package typeck

import (
	"fmt"
	. "rustgo/compiler/middle"
	"strings"
)

// Canonical query caching (per inference context).
//
// Trait selection and projection normalization over types that mention
// closures still under inference (their signatures hold inference
// variables) cannot use the global caches. Without a cache, chains of
// closure adapters (`it.map(f).map(g).filter(h)...`) are re-selected
// recursively at every level (`Map<I, F>: Iterator` needs `I: Iterator`
// several times: once per candidate match, once to commit, once per
// projection in the where-clauses), which is exponential in the chain
// length.
//
// Queries are keyed by their canonical form: inference variables are
// renumbered by first appearance and every unsealed closure is described by
// its (canonicalized) signature, kind and captures. The cached response
// records what the query did to the input variables plus its result, in
// terms of the canonical variables, and is replayed on a hit (rustc's
// canonical query responses, in miniature).

type canonicalizer struct {
	ic       *InferCtxt
	vars     map[Ty]Ty // unresolved inference variable -> canonical var
	orig     []Ty      // canonical index -> original variable
	seen     map[Ty]bool
	b        strings.Builder
	volatile bool
	// closures: the query mentions an unsealed closure
	closures bool
	// general: allow unresolved general (non-literal) variables. Selection
	// on a bare variable consults the pending obligations, which are not
	// part of the key.
	general bool
}

func (ic *InferCtxt) newCanonicalizer(general bool) *canonicalizer {
	return &canonicalizer{ic: ic, vars: map[Ty]Ty{}, general: general}
}

// canon folds a (resolved) type into its canonical form; with expand,
// unsealed closures' signatures are appended to the key.
func (c *canonicalizer) canon(a Ty, expand bool) Ty {
	return Fold(a, func(x Ty) Ty {
		switch x.Kind {
		case TInfer:
			if x.Index < 0 {
				c.volatile = true // already canonical (shouldn't happen)
				return x
			}
			if v, ok := c.vars[x]; ok {
				return v
			}
			if x.Sub == 0 && !c.general && expand {
				c.volatile = true
			}
			v := CanonicalVar(len(c.orig), x.Sub)
			c.vars[x] = v
			c.orig = append(c.orig, x)
			return v
		case TClosure:
			if expand && !c.ic.t.sealedClosure(x) {
				c.closures = true
				c.closure(x)
			}
		case TOpaque:
			// hidden types may still be revealed
			c.volatile = true
		}
		return nil
	})
}

func (c *canonicalizer) closure(x Ty) {
	if c.seen == nil {
		c.seen = map[Ty]bool{}
	}
	if c.seen[x] {
		return
	}
	c.seen[x] = true
	ci := c.ic.t.Closures[x]
	if ci == nil {
		c.volatile = true
		return
	}
	fmt.Fprintf(&c.b, "|%p:%d:%d(", x, ci.Kind, len(ci.Captures))
	for _, in := range ci.Inputs {
		c.arg(in)
	}
	c.b.WriteString(")->")
	c.arg(ci.Output)
	for _, cp := range ci.Captures {
		fmt.Fprintf(&c.b, ";%t%t", cp.ByRef, cp.Mut)
		c.arg(cp.Ty)
	}
}

// arg appends a type (resolved, canonicalized) to the key.
func (c *canonicalizer) arg(a Ty) {
	if a == nil {
		c.b.WriteString(",nil")
		return
	}
	a = c.ic.resolve(a)
	fmt.Fprintf(&c.b, ",%p", c.canon(a, true))
}

// queryKey builds the canonical key of a trait query (name is the assoc
// item for projections).
func (ic *InferCtxt) queryKey(kind string, trait *Def, args []Ty, name string, general bool) (*canonicalizer, string, bool) {
	c := ic.newCanonicalizer(general)
	fmt.Fprintf(&c.b, "%s%p", kind, trait)
	if ic.pickFirst {
		c.b.WriteString("!")
	}
	for _, a := range args {
		if a == nil {
			return nil, "", false
		}
		c.arg(a)
	}
	if name != "" {
		c.b.WriteString("::")
		c.b.WriteString(name)
	}
	if c.volatile {
		return nil, "", false
	}
	return c, c.b.String(), true
}

// canonResp is a cached query response.
type canonResp struct {
	vals []Ty // canonical values of the input variables after the query
	out  []Ty // canonical results
	n    int  // total canonical variables (inputs + fresh)
	sel  Selection
}

// response records the effect of a query on its input variables.
func (c *canonicalizer) response(out []Ty) *canonResp {
	r := &canonicalizer{ic: c.ic, vars: map[Ty]Ty{}, orig: append([]Ty{}, c.orig...)}
	for k, v := range c.vars {
		r.vars[k] = v
	}
	resp := &canonResp{}
	for _, v := range c.orig {
		resp.vals = append(resp.vals, r.canon(c.ic.resolve(v), false))
	}
	for _, o := range out {
		if o == nil {
			resp.out = append(resp.out, nil)
			continue
		}
		resp.out = append(resp.out, r.canon(c.ic.resolve(o), false))
	}
	if r.volatile {
		return nil
	}
	resp.n = len(r.orig)
	return resp
}

// replay applies a cached response to the current query's variables and
// returns its results (ok=false if it doesn't apply; nothing is changed
// then).
func (c *canonicalizer) replay(resp *canonResp) ([]Ty, bool) {
	ic := c.ic
	vars := make([]Ty, resp.n)
	copy(vars, c.orig)
	inst := func(t Ty) Ty {
		if t == nil || !t.HasInfer() {
			return t
		}
		return Fold(t, func(x Ty) Ty {
			if x.Kind == TInfer && x.Index < 0 {
				i := -(x.Index + 1)
				if vars[i] == nil {
					vars[i] = NewInfer(x.Sub)
				}
				return vars[i]
			}
			return nil
		})
	}
	s := ic.snapshot()
	for i, v := range resp.vals {
		if !ic.unify(c.orig[i], inst(v)) {
			ic.rollback(s)
			return nil, false
		}
	}
	out := make([]Ty, len(resp.out))
	for i, o := range resp.out {
		out[i] = inst(o)
	}
	return out, true
}

// closureQuery prepares the canonical cache lookup for a query that
// mentions unsealed closures (others are either globally cacheable or
// cheap). It returns the cached results on a hit, or the canonicalizer and
// key to record the response with on a miss (nil if not cacheable).
func (ic *InferCtxt) closureQuery(kind string, trait *Def, args []Ty, name string) (hit *canonResp, out []Ty, c *canonicalizer, key string) {
	c, key, ok := ic.queryKey(kind, trait, args, name, false)
	if !ok || !c.closures {
		return nil, nil, nil, ""
	}
	if resp, found := ic.queryCache[key]; found {
		if out, ok := c.replay(resp); ok {
			return resp, out, nil, ""
		}
		return nil, nil, nil, ""
	}
	return nil, nil, c, key
}

func (ic *InferCtxt) recordQuery(c *canonicalizer, key string, out []Ty, sel Selection) {
	resp := c.response(out)
	if resp == nil {
		return
	}
	resp.sel = Selection{Kind: sel.Kind, Impl: sel.Impl}
	if ic.queryCache == nil {
		ic.queryCache = map[string]*canonResp{}
	}
	ic.queryCache[key] = resp
}
