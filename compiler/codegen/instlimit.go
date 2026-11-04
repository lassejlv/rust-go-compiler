package codegen

import (
	"strconv"
	"strings"

	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// Polymorphic recursion (`fn f<T>(x: T) { f((x,)) }`, or a nested data
// type `enum N<T> { A(T), B(Box<N<Vec<T>>>) }` whose drop glue needs
// N<Vec<T>>) makes monomorphization unbounded. rustc stops it with its
// recursion limit; codegen bounds the nesting depth of instance arguments
// and of the types it defines instead, so such programs fail quickly
// instead of looping. The bound is generous (legitimate programs nest types
// to the depth of their iterator chains or type-level numbers): twice the
// crate's `#![recursion_limit]`, at least defaultInstDepth.
const defaultInstDepth = 400

func (g *Gen) instDepthLimit(mainDef *Def) int {
	limit := defaultInstDepth
	if mainDef == nil || mainDef.Crate == nil || mainDef.Crate.Ast == nil {
		return limit
	}
	for _, a := range mainDef.Crate.Ast.Attrs {
		if a.Path != "recursion_limit" {
			continue
		}
		for _, tt := range a.Tokens {
			if tt.Delim != 0 || tt.Tok.Kind != lexer.Str {
				continue
			}
			if n, err := strconv.Atoi(strings.Trim(tt.Tok.Text, `"`)); err == nil && 2*n > limit {
				limit = 2 * n
			}
		}
	}
	return limit
}

// tyDepth returns the nesting depth of t (memoized; types are interned).
func (g *Gen) tyDepth(t Ty) int {
	if t == nil {
		return 0
	}
	if d, ok := g.depthCache[t]; ok {
		return d
	}
	m := 0
	sub := func(x Ty) {
		if d := g.tyDepth(x); d > m {
			m = d
		}
	}
	sub(t.Elem)
	sub(t.Ret)
	for _, e := range t.Elems {
		sub(e)
	}
	for _, a := range t.Args {
		sub(a)
	}
	for _, b := range t.Bindings {
		sub(b.Ty)
	}
	g.depthCache[t] = m + 1
	return m + 1
}

func (g *Gen) checkInstDepth(d *Def, args []Ty) {
	if g.instLimit == 0 {
		return
	}
	for _, a := range args {
		if g.tyDepth(a) > g.instLimit {
			g.fatalf("%s: error: reached the recursion limit while instantiating `%s`", d.Span.Lo, clip(typeck.Instance{Def: d, Args: args}.String()))
		}
	}
}

func (g *Gen) checkTypeDepth(t Ty, drop bool) {
	limit := g.instLimit
	if !drop {
		// a little slack, so that the more precise instance and drop-glue
		// diagnostics come first
		limit += 8
	}
	if g.instLimit != 0 && g.tyDepth(t) > limit {
		if drop {
			g.fatalf("error[E0320]: overflow while adding drop-check rules for `%s`", clip(t.String()))
		}
		g.fatalf("error: reached the recursion limit while instantiating type `%s`", clip(t.String()))
	}
}

func clip(s string) string {
	if len(s) > 200 {
		return s[:200] + "..."
	}
	return s
}
