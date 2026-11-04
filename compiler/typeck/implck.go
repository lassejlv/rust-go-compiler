package typeck

// Item-level checks of impls (mirrors parts of rustc_hir_analysis):
// trait impls must implement exactly the trait's items with matching
// signatures (E0046, E0407, E0437, E0438, E0050, E0053, E0185, E0186,
// E0049, E0195), inherent impls of one type must not define an item twice
// (E0592), trait impls must respect the orphan rule (E0117) and must not
// overlap (E0119).
//
// Impls in std are not checked. Impls of std traits are only checked for
// the items rustgo's std mirrors (stdRequired, stdCompared): its traits do
// not mirror the real std's item by item.

import (
	"fmt"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkImpls() {
	// signatures of all functions are checked, even of unused ones (before impl
	// methods are compared with the trait's quietly)
	for _, d := range t.R.Defs {
		if d.Kind == DefFn && d.Fn != nil && d.Fn.Ast != nil && !t.isStd(d) && d.Intrinsic == "" {
			sig := t.FnSig(d)
			if out := d.Fn.Ast.Decl.Output; sig != nil && sig.Output != nil && out != nil && d.Fn.Ast.Body != nil && sig.Output.IsUnsized() {
				t.ErrorCode(out.GetSpan(), "E0277", "the size for values of type `%s` cannot be known at compilation time", sig.Output)
			}
			if sp := d.Fn.Ast.Decl.SelfParam; sp != nil && sp.Ty != nil && sig != nil && len(sig.Inputs) > 0 && d.Crate != nil && d.Crate.Local {
				// an explicit `self` type must mention `Self` (`self:
				// Box<Self>`, `self: &Rc<Self>`, ...)
				var selfTy Ty
				switch {
				case d.InTrait():
					selfTy = Param(0, "Self")
				case d.Parent != nil && d.Parent.Kind == DefImpl && d.Parent.Impl != nil:
					selfTy = d.Parent.Impl.SelfTy
				}
				if in := sig.Inputs[0]; selfTy != nil && in != nil && !in.HasError() && !selfTy.HasError() && in != selfTy && !mentionsParam(in, selfTy) {
					t.ErrorCode(sp.Ty.GetSpan(), "E0307", "invalid `self` parameter type: `%s`", in)
				}
			}
			if sig != nil && d.Crate != nil && d.Crate.Local {
				// the signature's types must be well-formed
				for _, ty := range append(append([]Ty{}, sig.Inputs...), sig.Output) {
					if bad := t.adtBoundViolation(ty); bad != nil {
						t.ErrorCode(d.Span, "E0277", "the trait bound `%s: %s` is not satisfied", bad.Args[0], bad.Trait.Name)
						break
					}
				}
			}
		}
	}
	for _, impl := range t.R.Impls {
		if t.isStd(impl) || impl.Impl == nil || impl.Impl.Ast == nil {
			continue
		}
		if impl.Impl.TraitRef != nil {
			if !impl.Impl.Negative {
				t.checkTraitImplItems(impl)
			}
			t.checkOrphan(impl)
		}
		t.checkImplRules(impl)
	}
	t.checkInherentDups()
	t.checkAdtDefs()
	t.checkBindingNames()
	t.checkLifetimes()
	t.checkGenericParams()
	// constants and statics are checked even when unused (their
	// evaluation errors and non-const calls are errors)
	for _, d := range t.R.Defs {
		if (d.Kind == DefConst || d.Kind == DefStatic) && !t.isStd(d) && d.Const != nil && d.Const.Expr != nil {
			t.checkConstReturn(d.Const.Expr)
		}
		if (d.Kind != DefConst && d.Kind != DefStatic) || t.isStd(d) || d.Const == nil || d.Const.Expr == nil || d.IsAssoc() {
			continue
		}
		if t.Generics(d).Count() != 0 {
			continue
		}
		t.checkUnusedConst(d)
		if d.Kind == DefStatic && !d.Const.Mut {
			t.checkStaticSync(d)
		}
		if d.Kind == DefConst {
			t.checkConstPanic(d)
		}
	}
}

// checkUnusedConst type checks a constant that may never be used (rustgo
// otherwise only evaluates constants) and runs the body checks on it. The
// type check is first done quietly; if it fails, it is repeated to report
// the errors.
func (t *Tcx) checkUnusedConst(d *Def) {
	key := InstKey(d, nil)
	if t.results[key] != nil || t.inProgress[key] {
		return
	}
	t.ConstTy(d) // (lowered loudly: lowering is cached)
	if r := t.quietCheck(d); r != nil {
		t.postCheck(r, d)
		return
	}
	// it does not type check: report why (like rustc, which checks every
	// body whether used or not)
	t.CheckInstance(d, nil)
}

// quietCheck type checks a non-generic body without reporting errors; it
// returns nil (and forgets the results) if that fails.
func (t *Tcx) quietCheck(d *Def) *Results {
	key := InstKey(d, nil)
	if r := t.results[key]; r != nil {
		return r
	}
	if t.inProgress[key] {
		return nil
	}
	before := t.quietErrs
	t.quiet++
	var r *Results
	func() {
		defer func() {
			if recover() != nil {
				r = nil
			}
		}()
		r = t.CheckInstance(d, nil)
	}()
	t.quiet--
	if r == nil || t.quietErrs != before {
		delete(t.results, key)
		return nil
	}
	return r
}

// fnHasSelfSized reports whether a trait item has `where Self: Sized`.
func whereSelfSized(g *ast.Generics) bool {
	if g == nil {
		return false
	}
	for _, w := range g.Where {
		pt, ok := w.Ty.(*ast.PathType)
		if !ok || pt.QSelf != nil || len(pt.Path.Segments) != 1 || pt.Path.Segments[0].Ident.Name != "Self" {
			continue
		}
		for _, b := range w.Bounds {
			if b.Path != nil && !b.Maybe && len(b.Path.Segments) > 0 && b.Path.Segments[len(b.Path.Segments)-1].Ident.Name == "Sized" {
				return true
			}
		}
	}
	return false
}

func itemKindWord(d *Def) string {
	switch d.Kind {
	case DefAssocTy:
		return "type"
	case DefConst:
		return "const"
	}
	return "method"
}

func (t *Tcx) checkTraitImplItems(impl *Def) {
	tr := impl.Impl.TraitRef.Trait
	if tr == nil || tr.Trait == nil {
		return
	}
	seen := map[string]bool{}
	for _, it := range impl.Impl.Order {
		key := fmt.Sprintf("%d/%s", it.Kind, it.Name)
		if it.Kind == DefConst || it.Kind == DefFn {
			key = "v/" + it.Name
		}
		if seen[key] {
			t.ErrorCode(it.Item.Name.Span, "E0201", "duplicate definitions with name `%s`", it.Name)
		}
		seen[key] = true
	}
	if t.isStd(tr) {
		t.checkStdTraitItems(impl, tr)
		t.checkStdExtraItems(impl, tr)
		if names, ok := stdCompared[tr.Name]; ok && tr.Parent != nil && tr.Parent.Name == names[0] {
			for _, it := range impl.Impl.Order {
				for _, n := range names[1:] {
					if ti := tr.Trait.Items[it.Name]; it.Name == n && it.Kind == DefFn && ti != nil && ti.Kind == DefFn && ti.Fn != nil && ti.Fn.Ast != nil {
						t.compareImplMethod(impl, it, ti)
					}
				}
			}
		}
		return
	}
	for _, it := range impl.Impl.Order {
		ti := tr.Trait.Items[it.Name]
		if ti != nil && ti.Kind != it.Kind {
			// the name is a trait item of another kind
			code := "E0324"
			switch it.Kind {
			case DefAssocTy:
				code = "E0325"
			case DefConst:
				code = "E0323"
			}
			t.ErrorCode(it.Item.Name.Span, code, "item `%s` is an associated %s, which doesn't match its trait `%s`", it.Name, itemKindWord(it), tr.Name)
			continue
		}
		if ti == nil {
			code := "E0407"
			switch it.Kind {
			case DefAssocTy:
				code = "E0437"
			case DefConst:
				code = "E0438"
			}
			t.ErrorCode(it.Item.Name.Span, code, "%s `%s` is not a member of trait `%s`", itemKindWord(it), it.Name, tr.Name)
			continue
		}
		if it.Kind == DefFn {
			t.compareImplMethod(impl, it, ti)
		}
		if it.Kind == DefConst {
			t.compareImplConst(impl, it, ti)
		}
	}
	var missing []string
	for _, ti := range tr.Trait.Order {
		if _, ok := impl.Impl.Items[ti.Name]; ok {
			continue
		}
		required := false
		switch ti.Kind {
		case DefFn:
			required = ti.Fn.Ast.Body == nil
		case DefConst:
			required = ti.Const.Expr == nil
		case DefAssocTy:
			required = ti.AssocTy.TyAst == nil
		}
		if required {
			missing = append(missing, "`"+ti.Name+"`")
		}
	}
	if len(missing) > 0 {
		sp := impl.Span
		t.ErrorCode(sp, "E0046", "not all trait items implemented, missing: %s", strings.Join(missing, ", "))
	}
}

func selfWord(sp *ast.SelfParam) string {
	switch sp.Kind {
	case ast.SelfRef:
		return "&self"
	case ast.SelfRefMut:
		return "&mut self"
	}
	return "self"
}

// compareImplMethod compares an impl method's signature with the trait's.
func (t *Tcx) compareImplMethod(impl, im, tm *Def) {
	idecl, tdecl := im.Fn.Ast.Decl, tm.Fn.Ast.Decl
	switch {
	case idecl.SelfParam != nil && tdecl.SelfParam == nil:
		t.ErrorCode(im.Item.Name.Span, "E0185", "method `%s` has a `%s` declaration in the impl, but not in the trait", im.Name, selfWord(idecl.SelfParam))
		return
	case idecl.SelfParam == nil && tdecl.SelfParam != nil:
		t.ErrorCode(im.Item.Name.Span, "E0186", "method `%s` has a `%s` declaration in the trait, but not in the impl", im.Name, selfWord(tdecl.SelfParam))
		return
	}
	if len(idecl.Params) != len(tdecl.Params) {
		n := len(idecl.Params)
		if idecl.SelfParam != nil {
			n++
		}
		m := len(tdecl.Params)
		if tdecl.SelfParam != nil {
			m++
		}
		t.ErrorCode(im.Item.Name.Span, "E0050", "method `%s` has %d parameter%s but the declaration in trait `%s::%s` has %d", im.Name, n, plural(n), impl.Impl.TraitRef.Trait.Name, tm.Name, m)
		return
	}
	// types: lowered quietly; anything not fully comparable is skipped
	ig, tg := t.Generics(im), t.Generics(tm)
	iown, town := ig.Own(), tg.Own()
	count := func(ps []*GenericParamDef, konst bool) int {
		n := 0
		for _, p := range ps {
			if p.Const == konst {
				n++
			}
		}
		return n
	}
	for _, konst := range []bool{false, true} {
		if n, m := count(iown, konst), count(town, konst); n != m {
			what := "type"
			if konst {
				what = "const"
			}
			t.ErrorCode(im.Item.Name.Span, "E0049", "method `%s` has %d %s parameter%s but its trait declaration has %d %s parameter%s", im.Name, n, what, plural(n), m, what, plural(m))
			return
		}
	}
	if n, ok := earlyBoundLifetimes(im.Fn.Ast); ok {
		if m, ok := earlyBoundLifetimes(tm.Fn.Ast); ok && n != m {
			t.ErrorCode(im.Item.Name.Span, "E0195", "lifetime parameters or bounds on method `%s` do not match the trait declaration", im.Name)
			return
		}
	}
	if len(iown) != len(town) {
		return
	}
	for i := range iown {
		if iown[i].Synthetic || town[i].Synthetic || iown[i].Const != town[i].Const {
			return
		}
	}
	trArgs := impl.Impl.TraitRef.Args
	if len(trArgs) != tg.OwnStart {
		return
	}
	var isig, tsig *FnSig
	ok := t.quietly(func() {
		isig = t.FnSig(im)
		tsig = t.FnSig(tm)
	})
	if !ok || isig == nil || tsig == nil || len(isig.Inputs) != len(tsig.Inputs) {
		return
	}
	args := append([]Ty{}, trArgs...)
	ident := ig.Identity()
	args = append(args, ident[ig.OwnStart:]...)
	same := func(a, b Ty) (equal, comparable bool) {
		var x, y Ty
		if !t.quietly(func() {
			x = t.Normalize(Subst(a, args))
			y = t.Normalize(b)
		}) {
			return false, false
		}
		if x == y {
			return true, true
		}
		for _, z := range []Ty{x, y} {
			if z == nil || z.HasProj() || z.HasOpaque() || z.HasError() || z.HasInfer() {
				return false, false
			}
		}
		return false, true
	}
	for i := range tsig.Inputs {
		if eq, cmp := same(tsig.Inputs[i], isig.Inputs[i]); cmp && !eq {
			t.ErrorCode(im.Item.Name.Span, "E0053", "method `%s` has an incompatible type for trait", im.Name)
			return
		}
	}
	if eq, cmp := same(tsig.Output, isig.Output); cmp && !eq {
		t.ErrorCode(im.Item.Name.Span, "E0053", "method `%s` has an incompatible type for trait", im.Name)
		return
	}
	t.checkStricterBounds(im, tm, ig, tg, args)
}

// checkStricterBounds reports a bound of an impl method's own type
// parameter that the trait method's declaration does not imply (E0276).
// Only bounds `P: Trait<..>` on the method's own parameters are compared.
func (t *Tcx) checkStricterBounds(im, tm *Def, ig, tg *Generics, args []Ty) {
	// (std's trait declarations in rustgo's library may carry fewer bounds
	// than the real ones)
	if im.Crate == nil || !im.Crate.Local || t.isStd(tm) {
		return
	}
	var tpreds []*TraitRef
	for _, q := range tg.Predicates {
		if q.Maybe || q.Trait == nil {
			continue
		}
		var s *TraitRef
		if !t.quietly(func() {
			s = q.Trait.Subst(args)
			na := make([]Ty, len(s.Args))
			for i, a := range s.Args {
				na[i] = t.Normalize(a)
			}
			s = &TraitRef{Trait: s.Trait, Args: na}
		}) || s == nil {
			return
		}
		for _, a := range s.Args {
			if a == nil || a.HasProj() || a.HasOpaque() || a.HasError() {
				return // not comparable
			}
		}
		tpreds = append(tpreds, s)
	}
	implied := func(p *TraitRef) bool {
		for _, q := range tpreds {
			var sups []*TraitRef
			if !t.quietly(func() { sups = t.AllSupertraits(q) }) {
				return true
			}
			for _, s := range append([]*TraitRef{q}, sups...) {
				if s.Trait != p.Trait || len(s.Args) != len(p.Args) {
					continue
				}
				eq := true
				for i := range s.Args {
					if s.Args[i] != p.Args[i] {
						eq = false
					}
				}
				if eq {
					return true
				}
			}
		}
		return false
	}
	for _, p := range ig.Predicates {
		if p.Maybe || p.Trait == nil || len(p.Trait.Args) == 0 || p.Trait.Trait == nil {
			continue
		}
		self := p.Trait.Args[0]
		if self == nil || self.Kind != TParam || self.Sub != 0 || self.Index < ig.OwnStart || self.Index >= len(ig.Params) {
			continue
		}
		if p.Trait.Trait.LangItem == "sized" {
			continue
		}
		ok := true
		for _, a := range p.Trait.Args {
			if a == nil || a.HasInfer() || a.HasError() || a.HasProj() || a.HasOpaque() {
				ok = false
			}
		}
		if !ok || implied(p.Trait) {
			continue
		}
		t.ErrorCode(im.Item.Name.Span, "E0276", "impl has stricter requirements than trait: `%s: %s`", self, p.Trait.Trait.Name)
		return
	}
}

// compareImplConst compares the type of an impl's associated const with
// the trait's (E0326).
func (t *Tcx) compareImplConst(impl, ic, tc *Def) {
	if ic.Const == nil || tc.Const == nil {
		return
	}
	trArgs := impl.Impl.TraitRef.Args
	if tg := t.Generics(tc); tg == nil || len(trArgs) != tg.OwnStart || tg.Count() != tg.OwnStart {
		return
	}
	var x, y Ty
	if !t.quietly(func() {
		x = t.Normalize(Subst(t.ConstTy(tc), trArgs))
		y = t.Normalize(t.ConstTy(ic))
	}) {
		return
	}
	if x == y {
		return
	}
	for _, z := range []Ty{x, y} {
		if z == nil || z.HasProj() || z.HasOpaque() || z.HasError() || z.HasInfer() {
			return
		}
	}
	t.ErrorCode(ic.Item.Name.Span, "E0326", "implemented const `%s` has an incompatible type for trait: expected `%s`, found `%s`", ic.Name, x, y)
}

// earlyBoundLifetimes counts the early-bound lifetime parameters of a
// method (rustc compares their numbers between an impl and its trait,
// E0195): those that appear in a bound or where-clause, or not in the
// parameter types. ok is false when that is not clear syntactically
// (projections, `impl Trait`, higher-ranked binders, fn pointers, macros
// in the types).
func earlyBoundLifetimes(fn *ast.FnItem) (n int, ok bool) {
	g := fn.Generics
	if g == nil {
		return 0, true
	}
	var names []string
	for _, p := range g.Params {
		if p.Kind == ast.GPLifetime {
			names = append(names, p.Name.Name)
		}
	}
	if len(names) == 0 {
		return 0, true
	}
	risky := false
	collect := func(into map[string]bool, n any) {
		if n == nil {
			return
		}
		bound := func(b *ast.Bound) {
			if b.Lifetime != "" {
				into[b.Lifetime] = true
			}
			if len(b.ForLifetimes) > 0 {
				risky = true
			}
		}
		ast.Walk(n, func(x any) bool {
			switch x := x.(type) {
			case *ast.RefType:
				if x.Lifetime != "" {
					into[x.Lifetime] = true
				}
			case *ast.PathType:
				if x.QSelf != nil {
					risky = true
				}
			case *ast.Path:
				for i, s := range x.Segments {
					if s.Args == nil {
						continue
					}
					for _, a := range s.Args.Args {
						if a.Lifetime != "" {
							into[a.Lifetime] = true
							if i > 0 && len(x.Segments) > 1 {
								risky = true // maybe `T::Assoc<'a>`
							}
						}
					}
				}
			case *ast.DynTraitType:
				for _, b := range x.Bounds {
					bound(b)
				}
			case *ast.ImplTraitType, *ast.FnPtrType, *ast.MacroType:
				risky = true
			}
			return true
		})
		if b, ok := n.(*ast.Bound); ok {
			bound(b)
		}
	}
	inWhere, inInputs := map[string]bool{}, map[string]bool{}
	for _, p := range g.Params {
		if p.Kind == ast.GPLifetime && len(p.Bounds) > 0 {
			inWhere[p.Name.Name] = true
		}
		for _, b := range p.Bounds {
			collect(inWhere, b)
			if b.Path != nil {
				collect(inWhere, b.Path)
			}
		}
	}
	for _, w := range g.Where {
		if w.Lifetime != "" {
			inWhere[w.Lifetime] = true
		}
		if len(w.ForLifetimes) > 0 {
			risky = true
		}
		if w.Ty != nil {
			collect(inWhere, w.Ty)
		}
		for _, b := range w.Bounds {
			collect(inWhere, b)
			if b.Path != nil {
				collect(inWhere, b.Path)
			}
		}
	}
	if sp := fn.Decl.SelfParam; sp != nil {
		if sp.Lifetime != "" {
			inInputs[sp.Lifetime] = true
		}
		if sp.Ty != nil {
			collect(inInputs, sp.Ty)
		}
	}
	for _, p := range fn.Decl.Params {
		collect(inInputs, p.Ty)
	}
	inOutput := map[string]bool{}
	if fn.Decl.Output != nil {
		collect(inOutput, fn.Decl.Output)
	}
	if risky {
		return 0, false
	}
	// (rustc's is_late_bound_map: early-bound when in a where-clause, or in
	// the output but not the inputs; an unused lifetime is late-bound)
	for _, nm := range names {
		if inWhere[nm] || (!inInputs[nm] && inOutput[nm]) {
			n++
		}
	}
	return n, true
}

// quietly runs f with error reporting suppressed; false if it reported
// errors or aborted.
func (t *Tcx) quietly(f func()) (ok bool) {
	before := t.quietErrs
	t.quiet++
	defer func() {
		t.quiet--
		if r := recover(); r != nil {
			ok = false
			return
		}
		ok = t.quietErrs == before
	}()
	f()
	return true
}

// isLocalTy reports whether ty counts as local to crate c for the orphan
// rule (local ADTs and dyn local traits, also behind the fundamental
// &, &mut, Box and Pin).
func (t *Tcx) isLocalTy(ty Ty, c *Crate) bool {
	switch ty.Kind {
	case TAdt:
		if ty.Def.Crate == c {
			return true
		}
		if (ty.Def.LangItem == "owned_box" || ty.Def.Name == "Pin") && len(ty.Args) > 0 {
			return t.isLocalTy(ty.Args[0], c)
		}
	case TRef:
		return t.isLocalTy(ty.Elem, c)
	case TDyn:
		return ty.Def != nil && ty.Def.Crate == c
	case TClosure, TParam, TInfer, TError, TProjection, TOpaque:
		return true // not modelled: assume fine
	}
	return false
}

// strictlyLocalTy is isLocalTy where type parameters are not local.
func (t *Tcx) strictlyLocalTy(ty Ty, c *Crate) bool {
	switch ty.Kind {
	case TAdt:
		if ty.Def.Crate == c {
			return true
		}
		if (ty.Def.LangItem == "owned_box" || ty.Def.Name == "Pin") && len(ty.Args) > 0 {
			return t.strictlyLocalTy(ty.Args[0], c)
		}
	case TRef:
		return t.strictlyLocalTy(ty.Elem, c)
	case TDyn:
		return ty.Def != nil && ty.Def.Crate == c
	}
	return false
}

func (t *Tcx) checkOrphan(impl *Def) {
	tr := impl.Impl.TraitRef
	if tr.Trait.Crate == impl.Crate {
		return
	}
	if t.Generics(impl).Count() != 0 {
		// a foreign trait implemented for an uncovered type parameter
		if len(tr.Args) > 0 && tr.Args[0].Kind == TParam && tr.Args[0].Sub == 0 {
			t.ErrorCode(impl.Span, "E0210", "type parameter `%s` must be used as the type parameter for some local type", tr.Args[0].Name)
			return
		}
		// no local type among the trait's arguments (`impl<T> Display
		// for Vec<T>`)
		if impl.Crate != nil && impl.Crate.Local {
			for _, a := range tr.Args {
				if a == nil || a.HasError() || a.HasProj() || a.HasOpaque() || a.HasInfer() || t.strictlyLocalTy(a, impl.Crate) {
					return
				}
				unmodeled := false
				Walk(a, func(x Ty) bool {
					if x.Kind == TClosure || (x.Kind == TParam && x.Sub != 0) {
						unmodeled = true
					}
					return !unmodeled
				})
				if unmodeled {
					return
				}
			}
			t.ErrorCode(impl.Span, "E0117", "only traits defined in the current crate can be implemented for types defined outside of the crate")
		}
		return
	}
	for _, a := range tr.Args {
		if a == nil || a.HasParam() || a.HasError() || t.isLocalTy(a, impl.Crate) {
			return
		}
	}
	t.ErrorCode(impl.Span, "E0117", "only traits defined in the current crate can be implemented for types defined outside of the crate")
}

// checkInherentDups reports items defined twice for the same type by
// non-generic inherent impls.
func (t *Tcx) checkInherentDups() {
	for _, impls := range t.inherent {
		seen := map[string]*Def{}
		for _, impl := range impls {
			if t.isStd(impl) || t.Generics(impl).Count() != 0 || impl.Impl.SelfTy == nil {
				continue
			}
			for _, it := range impl.Impl.Order {
				key := fmt.Sprintf("%d/%d/%s", impl.Impl.SelfTy.ID(), it.Kind, it.Name)
				if it.Kind == DefAssocTy {
					continue
				}
				if prev := seen[key]; prev != nil {
					t.ErrorCode(it.Item.Name.Span, "E0592", "duplicate definitions with name `%s`", it.Name)
					continue
				}
				seen[key] = it
			}
		}
	}
}

// blanketOverlap reports whether generic impl a (without where clauses)
// applies to every trait ref of impl b.
func (t *Tcx) blanketCovers(a, b *Def) bool {
	ga := t.Generics(a)
	var preds []*Predicate // bounds checked once the params are bound
	for _, p := range ga.Predicates {
		if p.Maybe || (p.Trait != nil && p.Trait.Trait.LangItem == "sized") {
			continue
		}
		if p.Trait == nil || len(p.Bindings) > 0 || len(p.Trait.Args) != 1 || p.Trait.Args[0].Kind != TParam {
			return false
		}
		preds = append(preds, p)
	}
	for _, p := range ga.Params {
		if p.Const {
			return false
		}
	}
	binds := map[int]Ty{}
	var match func(x, y Ty) bool
	match = func(x, y Ty) bool {
		if x.Kind == TParam && x.Index < len(ga.Params) && ga.Params[x.Index].Name == x.Name {
			if prev, ok := binds[x.Index]; ok {
				return prev == y
			}
			if (y.IsUnsized() && !ga.Params[x.Index].MaybeUnsized) || y.Kind == TParam {
				return false // implicit Sized bound / generic: be conservative
			}
			binds[x.Index] = y
			return true
		}
		if x.HasParam() || x.HasProj() || y.HasProj() || y.HasError() {
			if x.Kind != y.Kind {
				return false
			}
			switch x.Kind {
			case TRef, TPtr, TSlice:
				return x.Mut == y.Mut && match(x.Elem, y.Elem)
			case TAdt:
				if x.Def != y.Def || len(x.Args) != len(y.Args) {
					return false
				}
				for i := range x.Args {
					if !match(x.Args[i], y.Args[i]) {
						return false
					}
				}
				return true
			case TTuple:
				if len(x.Elems) != len(y.Elems) {
					return false
				}
				for i := range x.Elems {
					if !match(x.Elems[i], y.Elems[i]) {
						return false
					}
				}
				return true
			}
			return false
		}
		return x == y
	}
	ar, br := a.Impl.TraitRef.Args, b.Impl.TraitRef.Args
	if len(ar) != len(br) {
		return false
	}
	for i := range ar {
		if !match(ar[i], br[i]) {
			return false
		}
	}
	for _, p := range preds {
		self, ok := binds[p.Trait.Args[0].Index]
		if !ok || self.HasParam() || self.HasInfer() || !t.Implements(self, p.Trait.Trait) {
			return false
		}
	}
	return true
}

// checkImplObligations reports, for an impl without generic parameters,
// supertraits of the trait (and `where Self: ..` clauses) that the self
// type does not implement, and associated types that do not satisfy the
// bounds the trait declares on them (E0277).
func (t *Tcx) checkImplObligations(impl *Def) {
	ii := impl.Impl
	tr := ii.TraitRef.Trait
	if tr == nil || tr.Trait == nil || t.Generics(impl).Count() != 0 || impl.Crate == nil || !impl.Crate.Local {
		return
	}
	args := ii.TraitRef.Args
	for _, a := range args {
		if a == nil || a.HasParam() || a.HasError() || a.HasInfer() || a.HasProj() || a.HasOpaque() {
			return
		}
	}
	if len(args) != t.Generics(tr).Count() {
		return
	}
	holds := func(ref *TraitRef) (bool, bool) {
		for _, a := range ref.Args {
			if a == nil || a.HasParam() || a.HasError() || a.HasInfer() || a.HasProj() || a.HasOpaque() {
				return true, false
			}
		}
		ic := newInferCtxt(t)
		switch ic.evaluate(&Predicate{Trait: ref}, 0) {
		case SelNone:
			return false, true
		}
		return true, true
	}
	for _, st := range t.Supertraits(tr) {
		var ref *TraitRef
		if !t.quietly(func() {
			ref = st.Subst(args)
			for i, a := range ref.Args {
				ref.Args[i] = t.Normalize(a)
			}
		}) || ref == nil || ref.Trait == nil || ref.Trait.Trait == nil {
			continue
		}
		if ok, known := holds(ref); known && !ok {
			t.ErrorCode(impl.Span, "E0277", "the trait bound `%s: %s` is not satisfied", ref.Args[0], ref.Trait.Name)
			return
		}
	}
	for _, it := range ii.Order {
		if it.Kind != DefAssocTy {
			continue
		}
		ti := tr.Trait.Items[it.Name]
		if ti == nil || ti.Kind != DefAssocTy || ti.AssocTy == nil {
			continue
		}
		t.Generics(ti)
		if t.Generics(ti).Count() != len(args) {
			continue // generic associated type
		}
		for _, b := range ti.AssocTy.Bounds {
			if b.Maybe || b.Trait == nil || len(b.Bindings) > 0 {
				continue
			}
			var ref *TraitRef
			if !t.quietly(func() {
				ref = b.Trait.Subst(args)
				for i, a := range ref.Args {
					ref.Args[i] = t.Normalize(a)
				}
			}) || ref == nil {
				continue
			}
			if ok, known := holds(ref); known && !ok {
				sp := it.Span
				if it.Item != nil {
					sp = it.Item.Span
				}
				t.ErrorCode(sp, "E0277", "the trait bound `%s: %s` is not satisfied", ref.Args[0], ref.Trait.Name)
				return
			}
		}
	}
}

// genericImplsOverlap reports whether two generic impls of a trait
// certainly overlap: their trait refs unify (each impl's parameters taken
// as unknowns) and every where-clause then still constrains a parameter
// left unknown, so it cannot be refuted (rustc's coherence treats such
// ambiguous obligations as overlapping). Anything less clear-cut (bounds
// on concrete types, associated type bindings, const parameters) is not
// reported.
func (t *Tcx) genericImplsOverlap(a, b *Def) bool {
	ga, gb := t.Generics(a), t.Generics(b)
	for _, g := range []*Generics{ga, gb} {
		for _, p := range g.Params {
			if p.Const {
				return false
			}
		}
	}
	type term struct {
		ty   Ty
		side int
	}
	binds := [2]map[int]term{{}, {}}
	isParam := func(x term) bool {
		g := ga
		if x.side == 1 {
			g = gb
		}
		return x.ty.Kind == TParam && x.ty.Index < len(g.Params) && g.Params[x.ty.Index].Name == x.ty.Name
	}
	var walk func(x term) term
	walk = func(x term) term {
		for isParam(x) {
			b, ok := binds[x.side][x.ty.Index]
			if !ok {
				return x
			}
			x = b
		}
		return x
	}
	// occurs reports whether parameter p appears in y (through bindings)
	var occurs func(p, y term, d int) bool
	occurs = func(p, y term, d int) bool {
		if d > 50 {
			return true
		}
		y = walk(y)
		if isParam(y) {
			return y.side == p.side && y.ty.Index == p.ty.Index
		}
		if !y.ty.HasParam() {
			return false
		}
		var kids []Ty
		switch y.ty.Kind {
		case TRef, TPtr, TSlice, TArray:
			kids = []Ty{y.ty.Elem}
		case TAdt, TFnDef:
			kids = y.ty.Args
		case TTuple:
			kids = y.ty.Elems
		default:
			return true
		}
		for _, k := range kids {
			if occurs(p, term{k, y.side}, d+1) {
				return true
			}
		}
		return false
	}
	depth := 0
	var unify func(x, y term) bool
	unify = func(x, y term) bool {
		depth++
		if depth > 1000 {
			return false
		}
		x, y = walk(x), walk(y)
		if isParam(y) && !isParam(x) {
			x, y = y, x
		}
		if isParam(x) {
			if isParam(y) && x.side == y.side && x.ty.Index == y.ty.Index {
				return true
			}
			if !isParam(y) && occurs(x, y, 0) {
				return false
			}
			binds[x.side][x.ty.Index] = y
			return true
		}
		if x.ty.HasProj() || y.ty.HasProj() || x.ty.HasOpaque() || y.ty.HasOpaque() || x.ty.HasError() || y.ty.HasError() || x.ty.Kind == TParam || y.ty.Kind == TParam {
			return false
		}
		if !x.ty.HasParam() && !y.ty.HasParam() {
			return x.ty == y.ty
		}
		if x.ty.Kind != y.ty.Kind || x.ty.Mut != y.ty.Mut {
			return false
		}
		switch x.ty.Kind {
		case TRef, TPtr, TSlice:
			return unify(term{x.ty.Elem, x.side}, term{y.ty.Elem, y.side})
		case TAdt:
			if x.ty.Def != y.ty.Def || len(x.ty.Args) != len(y.ty.Args) {
				return false
			}
			for i := range x.ty.Args {
				if !unify(term{x.ty.Args[i], x.side}, term{y.ty.Args[i], y.side}) {
					return false
				}
			}
			return true
		case TTuple:
			if len(x.ty.Elems) != len(y.ty.Elems) {
				return false
			}
			for i := range x.ty.Elems {
				if !unify(term{x.ty.Elems[i], x.side}, term{y.ty.Elems[i], y.side}) {
					return false
				}
			}
			return true
		}
		return false
	}
	ar, br := a.Impl.TraitRef.Args, b.Impl.TraitRef.Args
	if len(ar) != len(br) {
		return false
	}
	for i := range ar {
		if !unify(term{ar[i], 0}, term{br[i], 1}) {
			return false
		}
	}
	for side, g := range []*Generics{ga, gb} {
		for _, p := range g.Predicates {
			if p.Maybe || p.Trait == nil {
				continue
			}
			if len(p.Bindings) > 0 {
				return false
			}
			if p.Trait.Trait.LangItem == "sized" {
				// an unknown type may be sized
				if x := walk(term{p.Trait.Args[0], side}); !isParam(x) && x.ty.IsUnsized() {
					return false
				}
				continue
			}
			for _, arg := range p.Trait.Args {
				if x := walk(term{arg, side}); !isParam(x) {
					return false
				}
			}
		}
	}
	return true
}

// stdRequired lists required items of std traits (rustgo's std does not
// mirror every trait item by item, so only these are checked). An entry
// applies only when the trait has all the listed items.
var stdRequired = map[string][]string{
	"Iterator":            {"Item", "next"},
	"DoubleEndedIterator": {"next_back"},
	"ExactSizeIterator":   {},
	"Display":             {"fmt"},
	"Debug":               {"fmt"},
	"Clone":               {"clone"},
	"Drop":                {"drop"},
	"Default":             {"default"},
	"Hash":                {"hash"},
	"PartialEq":           {"eq"},
	"PartialOrd":          {"partial_cmp"},
	"Ord":                 {"cmp"},
	"Deref":               {"Target", "deref"},
	"DerefMut":            {"deref_mut"},
	"Index":               {"Output", "index"},
	"IndexMut":            {"index_mut"},
	"From":                {"from"},
	"Into":                {"into"},
	"TryFrom":             {"Error", "try_from"},
	"AsRef":               {"as_ref"},
	"AsMut":               {"as_mut"},
	"FromStr":             {"Err", "from_str"},
	"IntoIterator":        {"Item", "IntoIter", "into_iter"},
	"FromIterator":        {"from_iter"},
	"Extend":              {"extend"},
	"Add":                 {"Output", "add"},
	"Sub":                 {"Output", "sub"},
	"Mul":                 {"Output", "mul"},
	"Div":                 {"Output", "div"},
	"Rem":                 {"Output", "rem"},
	"Neg":                 {"Output", "neg"},
	"Not":                 {"Output", "not"},
	"AddAssign":           {"add_assign"},
	"SubAssign":           {"sub_assign"},
	"MulAssign":           {"mul_assign"},
	"DivAssign":           {"div_assign"},
	"Future":              {"Output", "poll"},
}

// stdCompared lists std traits (with the name of their module in rustgo's
// std) whose listed methods have the real std's signatures, so impls of
// them are compared like impls of local traits (E0053, E0049, ...).
var stdCompared = map[string][]string{
	"Display":    {"fmt", "fmt"},
	"Debug":      {"fmt", "fmt"},
	"Iterator":   {"traits", "next"},
	"Clone":      {"clone", "clone", "clone_from"},
	"PartialEq":  {"cmp", "eq"},
	"PartialOrd": {"cmp", "partial_cmp"},
	"Ord":        {"cmp", "cmp"},
	"Hash":       {"hash", "hash"},
	"From":       {"convert", "from"},
	"Drop":       {"drop", "drop"},
	"Default":    {"default", "default"},
	"Deref":      {"deref", "deref"},
	"DerefMut":   {"deref", "deref_mut"},
	"FromStr":    {"traits", "from_str"},
	"Into":       {"convert", "into"},
	"AsRef":      {"convert", "as_ref"},
	"AsMut":      {"convert", "as_mut"},
	"Index":      {"index", "index"},
	"IndexMut":   {"index", "index_mut"},
	"Add":        {"arith", "add"},
	"Sub":        {"arith", "sub"},
	"Mul":        {"arith", "mul"},
	"Div":        {"arith", "div"},
	"Rem":        {"arith", "rem"},
	"Neg":        {"arith", "neg"},
	"AddAssign":  {"arith", "add_assign"},
	"SubAssign":  {"arith", "sub_assign"},
	"MulAssign":  {"arith", "mul_assign"},
	"DivAssign":  {"arith", "div_assign"},
	"RemAssign":  {"arith", "rem_assign"},
	"Not":        {"bit", "not"},
}

// stdExactItems lists std traits with all their items (rustc's): when
// rustgo's declaration has exactly these items, an impl item of another
// name is not a member of the trait (E0407, E0437, E0438).
var stdExactItems = map[string][]string{
	"Drop": {"drop"}, "Display": {"fmt"}, "Debug": {"fmt"}, "Default": {"default"},
	"Deref": {"Target", "deref"}, "DerefMut": {"deref_mut"},
	"AsRef": {"as_ref"}, "AsMut": {"as_mut"}, "From": {"from"}, "Into": {"into"},
	"Index": {"Output", "index"}, "IndexMut": {"index_mut"},
	"Neg": {"Output", "neg"}, "Not": {"Output", "not"},
	"Add": {"Output", "add"}, "Sub": {"Output", "sub"}, "Mul": {"Output", "mul"},
	"Div": {"Output", "div"}, "Rem": {"Output", "rem"},
	"AddAssign": {"add_assign"}, "SubAssign": {"sub_assign"}, "MulAssign": {"mul_assign"},
	"DivAssign": {"div_assign"}, "RemAssign": {"rem_assign"},
	"FromStr": {"Err", "from_str"},
}

// checkStdExtraItems reports impl items that are not members of a std trait
// whose items rustgo's std mirrors exactly.
func (t *Tcx) checkStdExtraItems(impl *Def, tr *Def) {
	want, ok := stdExactItems[tr.Name]
	if !ok || len(tr.Trait.Items) != len(want) || impl.Crate == nil || !impl.Crate.Local {
		return
	}
	for _, n := range want {
		if tr.Trait.Items[n] == nil {
			return
		}
	}
	for _, it := range impl.Impl.Order {
		if tr.Trait.Items[it.Name] != nil || it.Item == nil {
			continue
		}
		code := "E0407"
		switch it.Kind {
		case DefAssocTy:
			code = "E0437"
		case DefConst:
			code = "E0438"
		}
		t.ErrorCode(it.Item.Name.Span, code, "%s `%s` is not a member of trait `%s`", itemKindWord(it), it.Name, tr.Name)
	}
}

// checkStdTraitItems reports required items of well-known std traits
// missing from an impl (E0046).
func (t *Tcx) checkStdTraitItems(impl *Def, tr *Def) {
	req, ok := stdRequired[tr.Name]
	if !ok {
		return
	}
	if tr.Name == "Write" {
		return
	}
	for _, n := range req {
		if tr.Trait.Items[n] == nil {
			return // a different trait of that name
		}
	}
	var missing []string
	for _, n := range req {
		if _, ok := impl.Impl.Items[n]; !ok {
			missing = append(missing, "`"+n+"`")
		}
	}
	if len(missing) > 0 {
		t.ErrorCode(impl.Span, "E0046", "not all trait items implemented, missing: %s", strings.Join(missing, ", "))
	}
}

// checkImplRules checks rules of particular impls: Drop only for ADTs
// (E0120) and not together with Copy (E0184), Copy only when every field is
// Copy (E0204), no visibility on trait impl items (E0449), and every impl
// type or const parameter constrained by the impl header (E0207).
func (t *Tcx) checkImplRules(impl *Def) {
	ii := impl.Impl
	ia := ii.Ast
	if ia == nil {
		return
	}
	t.checkUnconstrained(impl)
	t.checkImplHeaderLifetimes(impl)
	if ii.TraitRef == nil && ia.Trait == nil && ii.SelfTy != nil && impl.Crate != nil && impl.Crate.Local {
		// inherent impls are only allowed for the crate's own types
		switch st := ii.SelfTy; st.Kind {
		case TAdt:
			if st.Def.Crate != impl.Crate {
				t.ErrorCode(impl.Span, "E0116", "cannot define inherent `impl` for a type outside of the crate where the type is defined")
			}
		case TDyn:
			if st.Def != nil && st.Def.Crate != impl.Crate {
				t.ErrorCode(impl.Span, "E0116", "cannot define inherent `impl` for a type outside of the crate where the type is defined")
			}
		case TParam:
			t.ErrorCode(impl.Span, "E0118", "no nominal type found for inherent implementation")
		case TBool, TChar, TInt, TFloat, TStr, TSlice, TArray, TRef, TPtr, TTuple, TFnPtr, TNever:
			t.ErrorCode(impl.Span, "E0390", "cannot define inherent `impl` for primitive types")
		}
	}
	if ii.TraitRef != nil && impl.Crate != nil && impl.Crate.Local {
		tr := ii.TraitRef.Trait
		switch {
		case ii.Negative:
			t.ErrorCode(impl.Span, "E0658", "negative trait bounds are not supported")
		case tr != nil && tr.LangItem == "sized":
			t.ErrorCode(impl.Span, "E0322", "explicit impls for the `Sized` trait are not permitted")
		case tr != nil && ii.SelfTy != nil && ii.SelfTy.Kind == TDyn && ii.SelfTy.Def != nil && t.autoKind(tr) == "":
			// `impl Trait for dyn Trait` (or for a subtrait object)
			obj := &TraitRef{Trait: ii.SelfTy.Def, Args: append([]Ty{ii.SelfTy}, ii.SelfTy.Args...)}
			var sups []*TraitRef
			if t.quietly(func() { sups = t.AllSupertraits(obj) }) {
				for _, s := range append([]*TraitRef{obj}, sups...) {
					if s.Trait == tr {
						t.ErrorCode(impl.Span, "E0371", "the object type `%s` automatically implements the trait `%s`", ii.SelfTy, tr.Name)
						break
					}
				}
			}
		}
	}
	if ii.TraitRef == nil || ii.Negative {
		return
	}
	for _, it := range ii.Order {
		if it.Item != nil && it.Item.Vis != ast.VisPrivate {
			t.ErrorCode(it.Item.Span, "E0449", "visibility qualifiers are not permitted here")
		}
	}
	tr := ii.TraitRef.Trait
	if tr != nil && tr.Item != nil && !impl.HasAttr("automatically_derived") {
		if ta, ok := tr.Item.Kind.(*ast.TraitItem); ok {
			switch {
			case ta.Unsafe && !ia.Unsafe:
				t.ErrorCode(impl.Span, "E0200", "the trait `%s` requires an `unsafe impl` declaration", tr.Name)
			case !ta.Unsafe && ia.Unsafe && !t.isStd(tr):
				// (only for non-std traits: rustgo's std may not mark
				// every unsafe trait of the real std as unsafe)
				t.ErrorCode(impl.Span, "E0199", "implementing the trait `%s` is not unsafe", tr.Name)
			}
		}
	}
	self := ii.SelfTy
	if self == nil {
		return
	}
	t.checkImplObligations(impl)
	switch tr {
	case t.LangOpt("drop"):
		if self.Kind != TAdt && self.Kind != TError && self.Kind != TParam {
			t.ErrorCode(impl.Span, "E0120", "the `Drop` trait may only be implemented for local structs, enums, and unions")
			return
		}
		if self.Kind == TAdt {
			if copyImpl := t.adtCopyImpl(self.Def); copyImpl != nil {
				t.ErrorCode(copyImpl.Span, "E0184", "the trait `Copy` cannot be implemented for this type; the type has a destructor")
			}
			t.checkDropImplParams(impl, self)
		}
	case t.LangOpt("copy"):
		if self.Kind != TAdt || t.Generics(impl).Count() != 0 || self.HasParam() {
			return
		}
		t.AdtFields(self.Def)
		for _, v := range self.Def.Adt.Variants {
			for _, f := range v.Fields {
				ft := Subst(f.Ty, self.Args)
				if ft.HasError() || ft.HasParam() || ft.HasProj() {
					continue
				}
				if !t.IsCopy(ft) && t.certainlyNotCopy(ft, impl.Crate) {
					sp := impl.Span
					if f.Ast != nil {
						sp = f.Ast.Span
					}
					t.ErrorCode(sp, "E0204", "the trait `Copy` cannot be implemented for this type: field `%s` does not implement `Copy`", f.Name)
					return
				}
			}
		}
	}
}

// adtCopyImpl returns a Copy impl of ADT d (or nil).
func (t *Tcx) adtCopyImpl(d *Def) *Def {
	cp := t.LangOpt("copy")
	if cp == nil {
		return nil
	}
	for _, impl := range t.R.Impls {
		if impl.Impl != nil && impl.Impl.TraitRef != nil && impl.Impl.TraitRef.Trait == cp && !impl.Impl.Negative &&
			impl.Impl.SelfTy != nil && impl.Impl.SelfTy.Kind == TAdt && impl.Impl.SelfTy.Def == d {
			return impl
		}
	}
	return nil
}

// checkUnconstrained reports impl type and const parameters that appear
// neither in the impl's self type or trait nor in an associated type
// binding of its bounds (E0207).
func (t *Tcx) checkUnconstrained(impl *Def) {
	ia := impl.Impl.Ast
	if ia.Generics == nil {
		return
	}
	named := map[string]bool{}
	collect := func(n any) {
		ast.Walk(n, func(x any) bool {
			switch y := x.(type) {
			case *ast.Path:
				for _, sg := range y.Segments {
					named[sg.Ident.Name] = true
				}
			case *ast.MacroType:
				named["*"] = true
			}
			return true
		})
	}
	collect(ia.SelfTy)
	if ia.Trait != nil {
		collect(ia.Trait)
	}
	var bindings func(n any)
	bindings = func(n any) {
		ast.Walk(n, func(x any) bool {
			if p, ok := x.(*ast.Path); ok {
				for _, sg := range p.Segments {
					if sg.Args != nil {
						if sg.Args.Parenthesized && sg.Args.Output != nil {
							collect(sg.Args.Output) // Fn(..) -> R binds Output = R
						}
						for _, b := range sg.Args.Bindings {
							if b.Ty != nil {
								collect(b.Ty)
							}
						}
					}
				}
			}
			return true
		})
	}
	for _, p := range ia.Generics.Params {
		for _, b := range p.Bounds {
			bindings(b)
		}
	}
	for _, w := range ia.Generics.Where {
		bindings(w)
	}
	if named["*"] {
		return
	}
	for _, p := range ia.Generics.Params {
		if p.Kind != ast.GPLifetime && !named[p.Name.Name] {
			kind := "type"
			if p.Kind == ast.GPConst {
				kind = "const"
			}
			t.ErrorCode(p.Name.Span, "E0207", "the %s parameter `%s` is not constrained by the impl trait, self type, or predicates", kind, p.Name.Name)
		}
	}
}

// checkConstReturn reports `return` in a constant's initializer (outside
// closures) (E0572).
func (t *Tcx) checkConstReturn(e ast.Expr) {
	ast.Walk(e, func(n any) bool {
		switch x := n.(type) {
		case *ast.ClosureExpr, *ast.ItemStmt:
			return false
		case *ast.ReturnExpr:
			t.ErrorCode(x.Span, "E0572", "return statement outside of function body")
		}
		return true
	})
}

// certainlyNotCopy reports types that are surely not Copy: &mut T, str,
// slices, std's owning containers, and ADTs of crate c without a Copy
// impl (types of other crates may be Copy through impls rustgo does not
// see the same way).
func (t *Tcx) certainlyNotCopy(ty Ty, c *Crate) bool {
	switch ty.Kind {
	case TRef:
		return ty.Mut
	case TStr, TSlice, TDyn:
		return true
	case TArray:
		return t.certainlyNotCopy(ty.Elem, c)
	case TTuple:
		for _, e := range ty.Elems {
			if t.certainlyNotCopy(e, c) {
				return true
			}
		}
		return false
	case TAdt:
		d := ty.Def
		if t.isStd(d) {
			switch d.Name {
			case "String", "Vec", "Box", "HashMap", "HashSet", "BTreeMap", "BTreeSet", "VecDeque", "Rc", "Arc", "RefCell", "Cell", "Mutex":
				return true
			}
			return false
		}
		return d.Crate == c && t.adtCopyImpl(d) == nil
	}
	return false
}

// checkStaticSync reports a (non-mut) static whose type is not Sync (E0277):
// it is shared between threads.
func (t *Tcx) checkStaticSync(d *Def) {
	ty := t.ConstTy(d)
	if ty == nil || ty.HasError() || ty.HasParam() || ty.HasProj() {
		return
	}
	if !newInferCtxt(t).autoHolds("Sync", ty, 0, map[autoKey]bool{}) {
		sp := d.Span
		if d.Const.TyAst != nil {
			sp = d.Const.TyAst.GetSpan()
		}
		t.ErrorCode(sp, "E0277", "`%s` cannot be shared between threads safely", ty)
	}
}

// checkDropImplParams reports `Drop` impls that are specialized: the self
// type's arguments must be distinct parameters of the impl (E0366), and
// the impl may not add where-clauses the type does not have (E0367).
func (t *Tcx) checkDropImplParams(impl *Def, self Ty) {
	if impl.Crate == nil || !impl.Crate.Local {
		return
	}
	ig := t.Generics(impl)
	seen := map[int]bool{}
	for _, a := range self.Args {
		if a == nil || a.Kind != TParam || a.Index >= len(ig.Params) || ig.Params[a.Index].Name != a.Name || seen[a.Index] {
			t.ErrorCode(impl.Span, "E0366", "`Drop` impls cannot be specialized")
			return
		}
		seen[a.Index] = true
	}
	ag := t.Generics(self.Def)
	var adtPreds []*TraitRef
	for _, p := range ag.Predicates {
		if p.Trait != nil && !p.Maybe {
			adtPreds = append(adtPreds, p.Trait.Subst(self.Args))
		}
	}
	for _, p := range ig.Predicates {
		if p.Maybe || p.Trait == nil || p.Trait.Trait == nil || p.Trait.Trait.LangItem == "sized" || len(p.Bindings) > 0 {
			continue
		}
		found := false
		for _, ap := range adtPreds {
			if ap.Trait != p.Trait.Trait || len(ap.Args) != len(p.Trait.Args) {
				continue
			}
			same := true
			for i := range ap.Args {
				if ap.Args[i] != p.Trait.Args[i] {
					same = false
				}
			}
			if same {
				found = true
				break
			}
		}
		if !found {
			// (also implied by a supertrait of a declared bound)
			for _, ap := range adtPreds {
				for _, st := range t.AllSupertraits(ap) {
					if st.Trait == p.Trait.Trait && len(st.Args) == len(p.Trait.Args) && st.Args[0] == p.Trait.Args[0] {
						found = true
					}
				}
			}
		}
		if !found {
			t.ErrorCode(impl.Span, "E0367", "`Drop` impl requires `%s: %s` but the type it is implemented for does not", p.Trait.Args[0], p.Trait.Trait.Name)
			return
		}
	}
}
