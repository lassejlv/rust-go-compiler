package typeck

// Item-level checks of impls (mirrors parts of rustc_hir_analysis):
// trait impls must implement exactly the trait's items with matching
// signatures (E0046, E0407, E0437, E0438, E0050, E0053, E0185, E0186),
// inherent impls of one type must not define an item twice (E0592), trait
// impls must respect the orphan rule (E0117) and must not overlap (E0119).
//
// Impls in std and impls of std traits are not checked: rustgo's std does
// not mirror the real std's traits item by item.

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
			t.FnSig(d)
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
		return
	}
	for _, it := range impl.Impl.Order {
		ti := tr.Trait.Items[it.Name]
		if ti == nil || ti.Kind != it.Kind {
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
	}
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

func (t *Tcx) checkOrphan(impl *Def) {
	tr := impl.Impl.TraitRef
	if tr.Trait.Crate == impl.Crate {
		return
	}
	if t.Generics(impl).Count() != 0 {
		// a foreign trait implemented for an uncovered type parameter
		if len(tr.Args) > 0 && tr.Args[0].Kind == TParam && tr.Args[0].Sub == 0 {
			t.ErrorCode(impl.Span, "E0210", "type parameter `%s` must be used as the type parameter for some local type", tr.Args[0].Name)
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
				key := fmt.Sprintf("%p/%d/%s", impl.Impl.SelfTy, it.Kind, it.Name)
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
			if y.IsUnsized() || y.Kind == TParam {
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
	if ii.TraitRef == nil || ii.Negative {
		return
	}
	for _, it := range ii.Order {
		if it.Item != nil && it.Item.Vis != ast.VisPrivate {
			t.ErrorCode(it.Item.Span, "E0449", "visibility qualifiers are not permitted here")
		}
	}
	tr := ii.TraitRef.Trait
	self := ii.SelfTy
	if self == nil {
		return
	}
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
				if !t.IsCopy(ft) {
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
