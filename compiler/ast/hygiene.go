package ast

import "sort"

// Hygiene of macro_rules! expansions ("mixed site" hygiene, like rustc):
// local variables and labels named by a macro body resolve where the
// macro is defined, items where it is invoked.
//
// Every token transcribed from a macro body gets a syntax context (its
// Hyg): one fresh context per expansion and per context the token had in
// the definition (0 for tokens written in the source). HygInfo describes
// a context: Parent is the context the body token had (the context with
// this expansion's mark removed) and Def the definition of the expanded
// macro when it is defined inside a block (the EmptyStmt.MacroDef marker
// left where the definition was; 0 for a macro defined at module level).
//
// A local binding is known by (name, context). Looking up an identifier
// walks the scopes outwards from the use; when the walk passes the marker
// of the definition that created the identifier's context, the context is
// replaced by its Parent: from there on the identifier is resolved like
// the token of the definition it was transcribed from.
type HygInfo struct {
	Parent uint32
	Def    uint32
}

var hygInfos = map[uint32]HygInfo{}

// SetHygInfo records the description of context ctx.
func SetHygInfo(ctx uint32, h HygInfo) { hygInfos[ctx] = h }

// HygInfoOf returns the description of context ctx (zero if none was
// recorded: a context of a macro defined at module level whose tokens had
// no context in the definition).
func HygInfoOf(ctx uint32) HygInfo { return hygInfos[ctx] }

// HygStrip returns the context of an identifier with context ctx once the
// lookup has walked past the marker of local macro definition def.
func HygStrip(ctx, def uint32) uint32 {
	if ctx != 0 && def != 0 {
		if h := hygInfos[ctx]; h.Def == def {
			return h.Parent
		}
	}
	return ctx
}

// HygInfos encodes the descriptions of the contexts created after the
// encoder's HygBase.
func (e *Encoder) HygInfos() {
	var ks []uint32
	for k := range hygInfos {
		if k > e.HygBase {
			ks = append(ks, k)
		}
	}
	sort.Slice(ks, func(i, j int) bool { return ks[i] < ks[j] })
	e.guard(func() {
		e.u(uint64(len(ks)))
		for _, k := range ks {
			h := hygInfos[k]
			e.hyg(k)
			e.hyg(h.Parent)
			e.hyg(h.Def)
		}
	})
}

// HygInfos decodes descriptions written by Encoder.HygInfos; install
// records them.
func (d *Decoder) HygInfos() (install func(), err error) {
	type ent struct {
		k uint32
		h HygInfo
	}
	var es []ent
	err = d.guard(func() {
		n := d.u()
		for i := uint64(0); i < n; i++ {
			k := d.hyg()
			p := d.hyg()
			def := d.hyg()
			es = append(es, ent{k, HygInfo{p, def}})
		}
	})
	if err != nil {
		return nil, err
	}
	return func() {
		for _, x := range es {
			hygInfos[x.k] = x.h
		}
	}, nil
}
