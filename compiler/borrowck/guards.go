package borrowck

import (
	"fmt"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// Match guards (E0510): a match with guards holds a shallow "fake"
// borrow of the places its patterns test while its guards run, so a guard
// may not assign the scrutinee (or a prefix of it) nor borrow it mutably.
// Only the scrutinee itself is modeled as a tested place, when it is a
// user variable (or a field path of one) of an enum or scalar type tested
// at the top of some arm's pattern (a variant, literal or range).

type guardRange struct {
	place      Place // the tested scrutinee
	block      int   // the guard's code: from stmt of block,
	stmt       int
	blocksFrom int // and the blocks numbered blocksFrom..blocksTo-1
	blocksTo   int
}

// guardTested returns the scrutinee place a match with guards tests (ok
// false when none is modeled).
func (b *builder) guardTested(x *ast.MatchExpr, src tplace) (Place, bool) {
	guarded := false
	for _, arm := range x.Arms {
		if arm.Guard != nil {
			guarded = true
		}
	}
	if !guarded || src.ty == nil || src.ty.Kind == TRef {
		return Place{}, false
	}
	ld := b.body.Locals[src.p.Local]
	if ld.Kind != LVar && ld.Kind != LArg {
		return Place{}, false
	}
	for _, pr := range src.p.Proj {
		if pr.Kind != PField {
			return Place{}, false
		}
	}
	for _, arm := range x.Arms {
		if patTestsTop(arm.Pat, src.ty) {
			return src.p, true
		}
	}
	return Place{}, false
}

// patTestsTop reports whether pattern p tests the value it matches itself
// (a discriminant or value comparison), for a value of type t.
func patTestsTop(p ast.Pat, t Ty) bool {
	multiVariant := t.Kind == TAdt && t.Def != nil && t.Def.Adt != nil && t.Def.Adt.IsEnum() && len(t.Def.Adt.Variants) >= 2
	switch x := p.(type) {
	case *ast.ParenPat:
		return patTestsTop(x.Pat, t)
	case *ast.OrPat:
		for _, a := range x.Alts {
			if patTestsTop(a, t) {
				return true
			}
		}
	case *ast.LitPat:
		return t.Kind == TInt || t.Kind == TChar || t.Kind == TBool
	case *ast.RangePat:
		return t.Kind == TInt || t.Kind == TChar
	case *ast.TupleStructPat, *ast.StructPat, *ast.PathPat:
		return multiVariant
	case *ast.IdentPat:
		if x.Sub != nil {
			return patTestsTop(x.Sub, t)
		}
		if multiVariant && !x.ByRef && !x.Mut {
			for _, v := range t.Def.Adt.Variants {
				if v.Name == x.Name.Name && v.Shape == ast.ShapeUnit {
					return true
				}
			}
		}
	}
	return false
}

// prefixOf reports whether place a is place p or a prefix of it (field
// projections only).
func prefixOf(a, p Place) bool {
	if a.Local != p.Local || len(a.Proj) > len(p.Proj) {
		return false
	}
	for i, pr := range a.Proj {
		q := p.Proj[i]
		if pr.Kind != PField || q.Kind != PField || pr.Field != q.Field || pr.Variant != q.Variant {
			return false
		}
	}
	return true
}

// checkGuards reports the assignments and mutable borrows of tested
// scrutinees in match guards.
func (b *builder) checkGuards() {
	for _, g := range b.guards {
		report := func(sp lexer.Span, p Place, what string) {
			name := b.body.Locals[p.Local].Name
			for _, pr := range p.Proj {
				if pr.Kind == PField && pr.Name != "" {
					name += "." + pr.Name
				}
			}
			b.body.Errors = append(b.body.Errors, fmt.Sprintf("%s: error[E0510]: cannot %s `%s` in match guard", sp.Lo, what, name))
		}
		visit := func(bl *Block, from int) {
			for _, s := range bl.Stmts[from:] {
				if s.Kind != SAssign {
					continue
				}
				if prefixOf(s.Place, g.place) {
					report(s.Span, s.Place, "assign")
					return
				}
				if s.Rv.Kind == RRef && s.Rv.Mut && prefixOf(s.Rv.Place, g.place) {
					report(s.Span, s.Rv.Place, "mutably borrow")
					return
				}
			}
			if t := bl.Term; t.Kind == TCall && prefixOf(t.Dest, g.place) {
				report(t.Span, t.Dest, "assign")
			}
		}
		if g.block >= 0 && g.block < len(b.body.Blocks) && g.stmt <= len(b.body.Blocks[g.block].Stmts) {
			visit(b.body.Blocks[g.block], g.stmt)
		}
		for bi := g.blocksFrom; bi < g.blocksTo && bi < len(b.body.Blocks); bi++ {
			visit(b.body.Blocks[bi], 0)
		}
	}
}
