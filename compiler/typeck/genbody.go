package typeck

// Polymorphic checks of generic bodies. rustgo type checks generic
// functions per instantiation (see the package comment), so an invalid
// generic body is accepted when it is never instantiated, or when every
// instance happens to work (`fn max<T>(a: T, b: T) -> T { if a > b ..`
// called with integers). rustc checks every body once, with the type
// parameters as opaque placeholders and the where-clauses as the only
// facts about them.
//
// CheckGenericBodies does the same check, quietly: rustgo's reasoning
// about type parameters (projections, autoderef through bounds, ...) is
// weaker than rustc's, so the errors of a body are only reported when every
// one of them is of a kind whose detection does not depend on that
// reasoning (classified where the error is found, see polyClassify):
//
//   - E0599: no method (or `T::item`) for a type parameter, or a reference
//     to one, whose bounds declare no item of that name and no Deref;
//   - E0369/E0368/E0600: an operator applied to a type parameter whose
//     bounds do not include the operator trait (only a where-clause can
//     provide one: an impl for all T of a foreign trait is not allowed);
//   - E0614: `*t` for a type parameter without a Deref bound;
//   - E0369 also for `&T == &T` (and `<`...) without `T: PartialEq`;
//   - E0308: a type parameter against another one or against a type with
//     a known constructor (`u8`, `&T`, `Vec<T>`; no unresolved parts), at
//     the top or under the same constructors (`Option<T>`/`Option<&T>`);
//   - E0277: an unsatisfied `T: Trait` where-clause of a called item of a
//     non-std crate, T a type parameter not bounded by Trait, and Trait
//     without impls for all types (`impl<T> Trait for T`);
//   - E0277: an unsatisfied where-clause of a called std item `T: Trait`
//     or `&T: Trait` for the std traits whose bounds rustgo's std mirrors
//     and that only a where-clause gives a type parameter (Display, Debug
//     and the other formatting traits, Hash, Eq, Ord; see polyStdTrait):
//     format arguments, `sort`, `HashMap::insert`; and the one failing
//     where-clause of a std collection's FromIterator/Extend impl;
//   - E0277: `Send`/`Sync` bounds of called functions on a type parameter
//     reached through references, tuples, closure captures (lacking both
//     traits then) and a few std containers (polyAutoBounds, after the
//     closures' capture analysis);
//   - E0277: a primitive scalar without Hash/Eq/Ord/a formatting trait
//     (`f32: Hash` in a derived `hash<H>`), and a `?Sized` type parameter
//     as a parameter or return type (polyUnsizedSig).
//
// Errors about erroneous types (`{error}`) and operator outputs after an
// unsatisfied operator bound are consequences of other errors. A body
// with any other error is not reported at all; nor are bodies of std or
// of non-local crates (dependencies).

import (
	"fmt"
	"os"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
)

// polyErr is an error recorded by a quiet polymorphic body check.
type polyErr struct {
	sp   lexer.Span
	code string // rustc's code when the error is reportable
	msg  string
	ok   bool // a whitelisted kind (see the file comment)
}

// polyState is the state of the polymorphic check of one body.
type polyState struct {
	def  *Def
	errs []polyErr
	next *polyErr // classification of the next error (set just before it)
}

// inPoly reports whether fcx is the polymorphic check of a generic body.
func (fcx *FnCtxt) inPoly() bool {
	return fcx.t.poly != nil && fcx.t.poly.def == fcx.def && fcx.t.quiet > 0
}

// polyClassify classifies the error reported next (by errorf) as a
// reportable one with rustc's code and message.
func (fcx *FnCtxt) polyClassify(code, format string, args ...any) {
	fcx.t.poly.next = &polyErr{code: code, msg: fmt.Sprintf(format, args...), ok: true}
}

// polyRecord records a quiet error during a polymorphic check.
func (t *Tcx) polyRecord(sp lexer.Span, text string) {
	e := polyErr{sp: sp, msg: text}
	if strings.Contains(text, "{error}") && len(t.poly.errs) > 0 {
		// about the type of an erroneous expression: a consequence of an
		// error recorded before
		e.ok = true
	}
	if t.poly.next != nil {
		e = *t.poly.next
		e.sp = sp
		t.poly.next = nil
	}
	t.poly.errs = append(t.poly.errs, e)
}

// CheckGenericBodies type checks the bodies of the generic functions of
// local crates polymorphically and reports the errors it is sure of.
func (t *Tcx) CheckGenericBodies() {
	mode := os.Getenv("RUSTGO_GENERICCK")
	if mode == "0" {
		return
	}
	for _, d := range t.R.Defs {
		if d.Kind != DefFn || d.Fn == nil || d.Fn.Ast == nil || d.Fn.Ast.Body == nil || d.Crate == nil {
			continue
		}
		if t.isStd(d) || (!d.Crate.Local && mode != "all") || d.Intrinsic != "" {
			continue
		}
		if t.Generics(d).Count() == 0 {
			continue
		}
		errs := t.polyCheck(d)
		if len(errs) == 0 {
			if r := t.polyResults; r != nil {
				// a body without errors: the deny-by-default lints
				t.polyResults = nil
				t.checkLints(r, d, d.Fn.Ast.Body)
				t.checkParamStatic(r, d)
				ast.Walk(d.Fn.Ast.Body, func(n any) bool {
					switch x := n.(type) {
					case *ast.ItemStmt:
						return false
					case *ast.UnaryExpr:
						t.checkNegUnsigned(r, x)
					}
					return true
				})
				// (`impl Trait` values of generic callees are not revealed
				// in a polymorphic check: their regions are not known well
				// enough to borrow-check the body)
				opaque := false
				for _, ty := range r.NodeTypes {
					if ty != nil && ty.HasOpaque() {
						opaque = true
						break
					}
				}
				for _, c := range r.Callees {
					if opaque {
						break
					}
					if c != nil && c.Def != nil && c.Def.Kind == DefFn && c.Def.Fn != nil {
						if sig := t.FnSig(c.Def); sig != nil && sig.Output != nil && sig.Output.HasOpaque() {
							opaque = true
						}
					}
				}
				if sig := t.FnSig(d); sig == nil || (sig.Output != nil && sig.Output.HasOpaque()) {
					opaque = true
				}
				if !opaque {
					r.PolyCopy = t.copyParams(d)
					t.PolyClean = append(t.PolyClean, PolyBody{Def: d, Results: r})
				}
			}
			continue
		}
		all := true
		for _, e := range errs {
			if !e.ok {
				all = false
			}
		}
		if os.Getenv("RUSTGO_TRACE_GENERICCK") != "" {
			for _, e := range errs {
				fmt.Fprintf(os.Stderr, "genericck %s (%v): %s: [%s] %s\n", d.Path(), all, e.sp.Lo, e.code, e.msg)
			}
		}
		if !all {
			continue
		}
		for _, e := range errs {
			if e.code != "" {
				t.ErrorCode(e.sp, e.code, "%s", e.msg)
			}
		}
	}
}

// PolyBody is a generic function whose polymorphic check found no error,
// with its results (borrow-checked when no instance was).
type PolyBody struct {
	Def     *Def
	Results *Results
}

// copyParams returns the type parameters of def with a Copy bound (or a
// bound with a Copy supertrait).
func (t *Tcx) copyParams(def *Def) map[int]bool {
	out := map[int]bool{}
	copyTr := t.LangOpt("copy")
	t.quietly(func() {
		for _, p := range t.Generics(def).Predicates {
			if p.Trait == nil || len(p.Trait.Args) == 0 || p.Maybe {
				continue
			}
			a := p.Trait.Args[0]
			if a.Kind != TParam {
				continue
			}
			for _, st := range t.AllSupertraits(p.Trait) {
				if st.Trait == copyTr {
					out[a.Index] = true
				}
			}
		}
	})
	return out
}

// polyCheck type checks def with its own parameters as types and returns
// the errors found (a non-reportable one if the check crashed).
func (t *Tcx) polyCheck(def *Def) (errs []polyErr) {
	ps := &polyState{def: def}
	saved := t.poly
	t.poly = ps
	t.quiet++
	defer func() {
		t.quiet--
		t.poly = saved
		if r := recover(); r != nil {
			errs = []polyErr{{sp: def.Span, msg: fmt.Sprintf("crash: %v", r)}}
		}
	}()
	fcx := t.newFnCtxt(def, t.Generics(def).Identity())
	t.polyUnsizedSig(def, ps)
	fcx.checkFn()
	t.polyResults = nil
	if len(ps.errs) == 0 && len(fcx.r.Closures) == 0 {
		fcx.writeback()
		if len(ps.errs) == 0 {
			t.polyResults = fcx.r
		}
		return ps.errs
	}
	if fcx.r.Body != nil && len(fcx.r.Closures) > 0 && polyAllOk(ps.errs) {
		// Send/Sync bounds need the closures' captures; errors of the
		// capture analysis itself only stop this check
		n := len(ps.errs)
		fcx.writeback()
		if len(ps.errs) == n {
			fcx.polyAutoBounds(fcx.r.Body)
			if len(ps.errs) == 0 {
				t.polyResults = fcx.r
			}
		} else {
			ps.errs = ps.errs[:n]
		}
	}
	return ps.errs
}

func polyAllOk(errs []polyErr) bool {
	for _, e := range errs {
		if !e.ok {
			return false
		}
	}
	return true
}

// paramFnSig returns the signature of a callable type parameter p from its
// `p: Fn*(..) -> R` where-clause (polymorphic checks only).
func (fcx *FnCtxt) paramFnSig(p Ty) ([]Ty, Ty, bool) {
	if !fcx.inPoly() {
		return nil, nil, false
	}
	isFn := func(tr *TraitRef) bool {
		switch tr.Trait.LangItem {
		case "fn", "fn_mut", "fn_once":
			return len(tr.Args) == 2 && tr.Args[0] == p && tr.Args[1].Kind == TTuple
		}
		return false
	}
	for _, e := range fcx.ic.envBindings {
		if isFn(e.Trait) {
			for _, b := range e.Bindings {
				if b.Name == "Output" {
					return e.Trait.Args[1].Elems, fcx.normalizeInBody(b.Ty, lexer.Span{}), true
				}
			}
		}
	}
	for _, e := range fcx.ic.env {
		if isFn(e) {
			return e.Args[1].Elems, TyUnit, true
		}
	}
	return nil, nil, false
}

// envMatches returns the distinct where-clauses (or their supertraits)
// that tr could be.
func (ic *InferCtxt) envMatches(tr *TraitRef) []*TraitRef {
	var out []*TraitRef
	add := func(st *TraitRef) {
		if st.Trait != tr.Trait || !ic.probeUnifyList(st.Args, tr.Args) {
			return
		}
		for _, o := range out {
			if sameArgs(o.Args, st.Args) {
				return
			}
		}
		out = append(out, st)
	}
	for _, e := range ic.env {
		if e.Trait == tr.Trait {
			add(e)
			continue
		}
		for _, st := range ic.t.AllSupertraits(e) {
			add(st)
		}
	}
	return out
}

// isTyParam reports whether x is a type (not const) parameter.
func isTyParam(x Ty) bool { return x != nil && x.Kind == TParam && x.Sub == 0 }

// mentionsParam reports whether x contains the parameter p.
func mentionsParam(x, p Ty) bool {
	found := false
	Walk(x, func(y Ty) bool {
		if y == p {
			found = true
		}
		return !found
	})
	return found
}

// paramBounds returns the where-clauses of the body (with their
// supertraits) whose self type mentions the type parameter p.
func (fcx *FnCtxt) paramBounds(p Ty) []*TraitRef {
	var out []*TraitRef
	for _, e := range fcx.ic.env {
		if len(e.Args) == 0 || !mentionsParam(e.Args[0], p) {
			continue
		}
		out = append(out, fcx.t.AllSupertraits(e)...)
	}
	return out
}

// concreteTy reports whether x is a fully known type without parameters
// (numeric literal variables allowed).
func (fcx *FnCtxt) concreteTy(x Ty) bool {
	x = fcx.resolve(x)
	ok := true
	Walk(x, func(y Ty) bool {
		switch y.Kind {
		case TParam, TProjection, TOpaque, TError, TNever, TClosure:
			ok = false
		case TInfer:
			if y.Sub == 0 {
				ok = false
			}
		}
		return ok
	})
	return ok
}

// rigidTy reports whether x is a type whose outermost constructor is known
// and is not a parameter (so it never equals a type parameter), with no
// unresolved or erroneous part.
func (fcx *FnCtxt) rigidTy(x Ty) bool {
	x = fcx.resolve(x)
	switch x.Kind {
	case TBool, TChar, TInt, TFloat, TStr, TAdt, TRef, TPtr, TSlice, TArray, TTuple, TFnPtr, TFnDef:
	case TInfer:
		return x.Sub != 0 // {integer} / {float}
	default:
		return false
	}
	ok := true
	Walk(x, func(y Ty) bool {
		switch y.Kind {
		case TError, TOpaque, TClosure, TDyn:
			ok = false
		case TInfer:
			if y.Sub == 0 {
				ok = false
			}
		}
		return ok
	})
	return ok
}

// opOutputTraits are the operator traits whose `Output` an operator
// expression's type is.
var opOutputTraits = map[string]bool{"add": true, "sub": true, "mul": true, "div": true, "rem": true,
	"bitand": true, "bitor": true, "bitxor": true, "shl": true, "shr": true, "neg": true, "not": true}

// unboundProj reports whether x contains the output of an operator trait
// that its (type parameter) self type is not bounded by: the trace of an
// unsatisfied operator obligation reported on its own (`<T as Add>::Output`
// after `t + 1` without `T: Add`; only a where-clause can provide an
// operator trait for a type parameter).
func (fcx *FnCtxt) unboundProj(x Ty) bool {
	found := false
	Walk(fcx.resolve(x), func(y Ty) bool {
		if y.Kind == TProjection && y.Def != nil && opOutputTraits[y.Def.LangItem] && len(y.Args) > 0 && isTyParam(y.Args[0]) {
			bounded := false
			for _, b := range fcx.paramBounds(y.Args[0]) {
				if b.Trait == y.Def {
					bounded = true
				}
			}
			if !bounded {
				found = true
			}
		}
		return !found
	})
	return found
}

// polyNoItem classifies a failed method lookup on recv, or a failed
// `T::item` path (assoc) (E0599): recv is a type parameter, or a reference
// to one, whose bounds declare no item of that name and include no Deref.
func (fcx *FnCtxt) polyNoItem(recv Ty, name string, assoc bool) {
	if !fcx.inPoly() {
		return
	}
	r := fcx.resolve(recv)
	p := r
	what := "type parameter"
	if p.Kind == TRef && !assoc {
		p = fcx.resolve(p.Elem)
		what = "reference"
	}
	if !isTyParam(p) {
		if !assoc && fcx.polyConcrete(r) {
			// a receiver type without parameters: the lookup does not
			// depend on the body's bounds
			fcx.polyClassify("E0599", "no method named `%s` found for `%s` in the current scope", name, r)
		}
		return
	}
	for _, b := range fcx.paramBounds(p) {
		if _, ok := b.Trait.Trait.Items[name]; ok {
			return
		}
		switch b.Trait.LangItem {
		case "deref", "deref_mut", "receiver":
			return
		}
	}
	if assoc {
		fcx.polyClassify("E0599", "no function or associated item named `%s` found for %s `%s` in the current scope", name, what, r)
		return
	}
	fcx.polyClassify("E0599", "no method named `%s` found for %s `%s` in the current scope", name, what, r)
}

// polyBinOp classifies an unsatisfied operator obligation (E0369/E0368):
// the left operand is a type parameter whose bounds do not include the
// operator trait.
func (fcx *FnCtxt) polyBinOp(o *Obligation, tr *TraitRef) {
	if !fcx.inPoly() || len(tr.Args) == 0 {
		return
	}
	lhs := fcx.resolve(tr.Args[0])
	if lhs.Kind == TRef && o.OpKind == opBinary && len(tr.Args) == 2 && (tr.Trait.LangItem == "eq" || tr.Trait.LangItem == "partial_ord") {
		// `&T == &T`: `&A: PartialEq<&B>` (and PartialOrd) hold exactly
		// when `A: PartialEq<B>`
		l, r := lhs, fcx.resolve(tr.Args[1])
		for l.Kind == TRef && r.Kind == TRef {
			l, r = fcx.resolve(l.Elem), fcx.resolve(r.Elem)
		}
		if !isTyParam(l) || r.Kind == TRef || l.Kind == TRef || !(isTyParam(r) || fcx.rigidTy(r)) {
			return
		}
		for _, b := range fcx.paramBounds(l) {
			if b.Trait == tr.Trait {
				return
			}
		}
		fcx.polyClassify("E0369", "binary operation `%s` cannot be applied to type `%s`", o.Op, lhs)
		return
	}
	if !isTyParam(lhs) {
		return
	}
	for _, b := range fcx.paramBounds(lhs) {
		if b.Trait == tr.Trait {
			return
		}
	}
	rhs := lhs
	if len(tr.Args) > 1 {
		rhs = fcx.resolve(tr.Args[1])
	}
	switch o.OpKind {
	case opAssign:
		fcx.polyClassify("E0368", "binary assignment operation `%s=` cannot be applied to type `%s`", o.Op, lhs)
	case opUnary:
		fcx.polyClassify("E0600", "cannot apply unary operator `%s` to type `%s`", o.Op, lhs)
	default:
		switch o.Op {
		case "+":
			fcx.polyClassify("E0369", "cannot add `%s` to `%s`", rhs, lhs)
		case "-":
			fcx.polyClassify("E0369", "cannot subtract `%s` from `%s`", rhs, lhs)
		case "*":
			fcx.polyClassify("E0369", "cannot multiply `%s` by `%s`", lhs, rhs)
		case "/":
			fcx.polyClassify("E0369", "cannot divide `%s` by `%s`", lhs, rhs)
		case "%":
			fcx.polyClassify("E0369", "cannot calculate the remainder of `%s` divided by `%s`", lhs, rhs)
		case "&", "|", "^", "<<", ">>":
			fcx.polyClassify("E0369", "no implementation for `%s %s %s`", lhs, o.Op, rhs)
		default:
			fcx.polyClassify("E0369", "binary operation `%s` cannot be applied to type `%s`", o.Op, lhs)
		}
	}
}

// Obligation.OpKind values.
const (
	opBinary = iota
	opAssign
	opUnary
)

func opKindOf(table map[ast.BinOp][2]string) int {
	if table[ast.BinAdd][0] == "add_assign" {
		return opAssign
	}
	return opBinary
}

// polyDeref classifies `*t` for a type parameter without a Deref bound
// (E0614).
func (fcx *FnCtxt) polyDeref(t Ty) {
	if !fcx.inPoly() {
		return
	}
	p := fcx.resolve(t)
	if !isTyParam(p) {
		return
	}
	for _, b := range fcx.paramBounds(p) {
		switch b.Trait.LangItem {
		case "deref", "deref_mut", "receiver":
			return
		}
	}
	fcx.polyClassify("E0614", "type `%s` cannot be dereferenced", p)
}

// polyMismatch classifies a type mismatch (E0308): a type parameter
// against a type with a known other constructor, or another parameter.
// Mismatches involving an operator's output after an unsatisfied operator
// bound are its consequence (not reported).
func (fcx *FnCtxt) polyMismatch(expected, actual Ty) {
	if !fcx.inPoly() || expected == nil || actual == nil {
		return
	}
	e, a := fcx.resolve(expected), fcx.resolve(actual)
	if fcx.polyParamMismatch(e, a, 0) || fcx.polyConcreteClash(e, a) {
		fcx.polyClassify("E0308", "mismatched types: expected `%s`, found `%s`", e, a)
		return
	}
	if fcx.unboundProj(e) || fcx.unboundProj(a) {
		fcx.t.poly.next = &polyErr{ok: true} // (consequence)
	}
}

// polyConcrete reports whether x is a fully known type without type
// parameters, projections, inference variables, closures or trait objects.
func (fcx *FnCtxt) polyConcrete(x Ty) bool {
	x = fcx.resolve(x)
	if x == nil || !fcx.rigidTy(x) || x.HasParam() || x.HasProj() || x.HasOpaque() || x.HasError() {
		return false
	}
	return true
}

// polyConcreteClash reports whether e and a are types without type
// parameters (nor projections, closures, trait objects, ...) with different
// outermost constructors, which no coercion relates (`i32` against
// `&str`): a mismatch that does not depend on the bounds of the body.
func (fcx *FnCtxt) polyConcreteClash(e, a Ty) bool {
	if !fcx.rigidTy(e) || !fcx.rigidTy(a) || e.HasParam() || a.HasParam() || e.HasProj() || a.HasProj() {
		return false
	}
	class := func(x Ty) (TyKind, int) {
		switch {
		case x.Kind == TInfer && x.Sub == 1:
			return TInt, -1
		case x.Kind == TInfer && x.Sub == 2:
			return TFloat, -1
		case x.Kind == TInt || x.Kind == TFloat:
			return x.Kind, int(x.Sub)
		case x.Kind == TAdt:
			return x.Kind, x.Def.ID
		}
		return x.Kind, -1
	}
	ek, es := class(e)
	ak, as := class(a)
	if ek != ak {
		// (fn items coerce to fn pointers)
		return !(ek == TFnPtr && ak == TFnDef)
	}
	return es != as && es != -1 && as != -1
}

// polyParamMismatch reports whether e and a certainly differ because of a
// type parameter: a parameter against a rigid type or another parameter,
// at the top or under the same constructors (`Option<T>` against
// `Option<&T>`).
func (fcx *FnCtxt) polyParamMismatch(e, a Ty, depth int) bool {
	e, a = fcx.resolve(e), fcx.resolve(a)
	if (isTyParam(e) && (fcx.rigidTy(a) || (isTyParam(a) && a != e))) || (isTyParam(a) && fcx.rigidTy(e)) {
		return true
	}
	if depth > 8 || e.Kind != a.Kind {
		return false
	}
	switch e.Kind {
	case TAdt:
		if e.Def != a.Def || len(e.Args) != len(a.Args) {
			return false
		}
		for i := range e.Args {
			if fcx.polyParamMismatch(e.Args[i], a.Args[i], depth+1) {
				return true
			}
		}
	case TRef:
		return e.Mut == a.Mut && fcx.polyParamMismatch(e.Elem, a.Elem, depth+1)
	case TSlice:
		return fcx.polyParamMismatch(e.Elem, a.Elem, depth+1)
	case TTuple:
		if len(e.Elems) != len(a.Elems) {
			return false
		}
		for i := range e.Elems {
			if fcx.polyParamMismatch(e.Elems[i], a.Elems[i], depth+1) {
				return true
			}
		}
	}
	return false
}

// polyUnsatisfied classifies an unsatisfied where-clause of a called item
// of a non-std crate (E0277): `T: Trait` for a type parameter T whose
// bounds do not mention Trait, where no impl of Trait applies to type
// parameters (only a where-clause could make it hold). Where-clauses of
// std items are handled by polyUnsatisfiedStd.
func (fcx *FnCtxt) polyUnsatisfied(o *Obligation, tr *TraitRef) {
	if !fcx.inPoly() || o.Depth != 0 || len(tr.Args) == 0 {
		return
	}
	if len(tr.Args) == 1 && fcx.t.polyStdTrait(tr.Trait) {
		// a primitive scalar without the trait (`f32: Hash` in a derived
		// `Hash::hash<H>`)
		switch self := fcx.resolve(tr.Args[0]); self.Kind {
		case TBool, TChar, TInt, TFloat:
			fcx.polyClassify("E0277", "%s", polyStdBoundMsg(self, tr.Trait))
			return
		}
	}
	if o.From == nil {
		return
	}
	if fcx.t.isStd(o.From) {
		fcx.polyUnsatisfiedStd(tr)
		return
	}
	self := fcx.resolve(tr.Args[0])
	trait := tr.Trait
	if !isTyParam(self) || trait.Trait == nil || trait.Trait.Auto || fcx.t.autoKind(trait) != "" {
		return
	}
	switch trait.LangItem {
	case "":
	case "clone", "copy", "eq", "partial_ord", "add", "sub", "mul", "div", "rem", "neg", "not":
	default:
		return // Sized, Fn*, ...: built-in rules
	}
	for _, a := range tr.Args[1:] {
		if !fcx.rigidTy(a) && !isTyParam(fcx.resolve(a)) {
			return
		}
	}
	for _, b := range fcx.paramBounds(self) {
		if b.Trait == trait {
			return
		}
	}
	for _, impl := range fcx.t.traitImpls[trait] {
		if impl.Impl.TraitRef.Args[0].Kind == TParam && !fcx.t.fnPtrBlanket(impl) {
			return // a blanket impl
		}
	}
	fcx.polyClassify("E0277", "the trait bound `%s: %s` is not satisfied", self, traitRefDisplay(&TraitRef{Trait: trait, Args: fcx.ic.resolveList(tr.Args)}))
}

// polyStdTrait reports whether tr is one of the std traits whose
// where-clauses on std items rustgo's std mirrors and that a type
// parameter only implements by a where-clause (no impl for all T): the
// formatting traits, Hash, Eq and Ord. `&T` and `&mut T` implement each
// of them exactly when T does.
func (t *Tcx) polyStdTrait(tr *Def) bool {
	if tr == nil || tr.Trait == nil || !t.isStd(tr) || tr.Parent == nil {
		return false
	}
	switch tr.Parent.Name {
	case "fmt":
		switch tr.Name {
		case "Display", "Debug", "LowerHex", "UpperHex", "Octal", "Binary", "LowerExp", "UpperExp":
			return true
		}
	case "hash":
		return tr.Name == "Hash"
	case "cmp":
		return tr.Name == "Eq" || tr.Name == "Ord"
	}
	return false
}

// polyStdBoundMsg is rustc's message for an unsatisfied `p: tr`.
func polyStdBoundMsg(p Ty, tr *Def) string {
	switch tr.Name {
	case "Display":
		return fmt.Sprintf("`%s` doesn't implement `std::fmt::Display`", p)
	case "Debug":
		return fmt.Sprintf("`%s` doesn't implement `Debug`", p)
	}
	return fmt.Sprintf("the trait bound `%s: %s` is not satisfied", p, tr.Name)
}

// polyParamLacks reports whether the type parameter p certainly does not
// implement the std trait tr (see polyStdTrait): none of the where-clauses
// mentioning p is tr or has it as a supertrait, and no impl of tr applies
// to all types.
func (fcx *FnCtxt) polyParamLacks(p Ty, tr *Def) bool {
	if !isTyParam(p) || !fcx.t.polyStdTrait(tr) {
		return false
	}
	for _, b := range fcx.paramBounds(p) {
		if b.Trait == tr {
			return false
		}
	}
	for _, impl := range fcx.t.traitImpls[tr] {
		if a := impl.Impl.TraitRef.Args; len(a) == 0 || a[0].Kind == TParam && !fcx.t.fnPtrBlanket(impl) {
			return false // a blanket impl
		}
	}
	return true
}

// polyUnsatisfiedStd classifies an unsatisfied where-clause of a called
// std item (E0277): `T: Trait` (or `&T: Trait`) for a std trait of
// polyStdTrait and a type parameter T not bounded by it (`T: Display` of
// a format argument, `T: Ord` of `sort`, `K: Eq + Hash` of
// `HashMap::insert`), or `Collection<..>: FromIterator<..>`/`Extend<..>`
// for a std collection whose only applicable impl has such an unsatisfied
// where-clause (`collect::<HashSet<T>>()` without `T: Eq`).
func (fcx *FnCtxt) polyUnsatisfiedStd(tr *TraitRef) {
	self := fcx.resolve(tr.Args[0])
	if self.Kind == TAdt {
		fcx.polyCollectionBound(self, tr)
		return
	}
	if len(tr.Args) != 1 {
		return
	}
	p := self
	for p.Kind == TRef {
		p = fcx.resolve(p.Elem)
	}
	if fcx.polyParamLacks(p, tr.Trait) {
		fcx.polyClassify("E0277", "%s", polyStdBoundMsg(p, tr.Trait))
	}
}

// polyCollections are the std collections whose FromIterator and Extend
// impls (and their where-clauses) rustgo's std mirrors.
var polyCollections = map[string]bool{"HashMap": true, "HashSet": true, "BTreeMap": true, "BTreeSet": true, "BinaryHeap": true}

// polyCollectionBound: see polyUnsatisfiedStd.
func (fcx *FnCtxt) polyCollectionBound(self Ty, tr *TraitRef) {
	t, ic := fcx.t, fcx.ic
	if !t.isStd(self.Def) || !polyCollections[self.Def.Name] || !t.isStd(tr.Trait) || (tr.Trait.Name != "FromIterator" && tr.Trait.Name != "Extend") {
		return
	}
	args := ic.resolveList(tr.Args)
	for _, a := range args {
		if a.HasInfer() || a.HasProj() || a.HasError() || a.HasOpaque() {
			return
		}
	}
	fresh := func(impl *Def) []Ty {
		v := make([]Ty, len(t.Generics(impl).Params))
		for i := range v {
			v[i] = ic.newVar()
		}
		return v
	}
	var cand *Def
	for _, impl := range t.traitImpls[tr.Trait] {
		if s := impl.Impl.TraitRef.Args[0]; s.Kind != TAdt || s.Def != self.Def {
			continue
		}
		s := ic.snapshot()
		ok := ic.unifyList(impl.Impl.TraitRef.Subst(fresh(impl)).Args, args)
		ic.rollback(s)
		if ok {
			if cand != nil {
				return
			}
			cand = impl
		}
	}
	if cand == nil {
		return
	}
	s := ic.snapshot()
	defer ic.rollback(s)
	iargs := fresh(cand)
	if !ic.unifyList(cand.Impl.TraitRef.Subst(iargs).Args, args) {
		return
	}
	var bad Ty
	var badTr *Def
	for _, p := range t.Generics(cand).Predicates {
		if p.Maybe || p.Trait.Trait.LangItem == "sized" {
			continue
		}
		sp := p.Subst(iargs)
		sp.Trait.Args = ic.resolveList(sp.Trait.Args)
		if ic.evaluate(sp, 1) != SelNone {
			continue
		}
		ps := fcx.resolve(sp.Trait.Args[0])
		if len(sp.Trait.Args) != 1 || len(sp.Bindings) > 0 || !fcx.polyParamLacks(ps, sp.Trait.Trait) {
			return // a failure not understood
		}
		if bad == nil {
			bad, badTr = ps, sp.Trait.Trait
		}
	}
	if bad != nil {
		fcx.polyClassify("E0277", "%s", polyStdBoundMsg(bad, badTr))
	}
}

// polyUnsizedSig records (E0277) parameters and return types of a body
// that are a type parameter declared `?Sized` and not bounded by Sized
// otherwise (`fn make<T: ?Sized>(t: Box<T>) -> T`).
func (t *Tcx) polyUnsizedSig(def *Def, ps *polyState) {
	g := t.Generics(def)
	decl := def.Fn.Ast.Decl
	sig := t.FnSig(def)
	unsized := func(x Ty) bool {
		if !isTyParam(x) || x.Index >= len(g.Params) {
			return false
		}
		gp := g.Params[x.Index]
		if gp.Name != x.Name || !gp.MaybeUnsized || gp.Const || gp.Synthetic || (g.HasSelf && x.Index == 0) || gp.Name == "Self" {
			return false
		}
		for _, p := range g.Predicates {
			if p.Maybe || p.Trait == nil || len(p.Trait.Args) == 0 || p.Trait.Args[0] != x {
				continue
			}
			for _, st := range t.AllSupertraits(p.Trait) {
				if st.Trait.LangItem == "sized" {
					return false
				}
			}
		}
		return true
	}
	add := func(sp lexer.Span, x Ty) {
		ps.errs = append(ps.errs, polyErr{sp: sp, code: "E0277", ok: true,
			msg: fmt.Sprintf("the size for values of type `%s` cannot be known at compilation time", x)})
	}
	off := 0
	if decl.SelfParam != nil {
		off = 1
	}
	if sig == nil || len(sig.Inputs) != len(decl.Params)+off {
		return
	}
	for i, p := range decl.Params {
		if x := sig.Inputs[i+off]; unsized(x) && p.Ty != nil {
			add(p.Ty.GetSpan(), x)
		}
	}
	if decl.Output != nil && unsized(sig.Output) {
		add(decl.Output.GetSpan(), sig.Output)
	}
}

// polyAutoBounds checks, once the captures of the body's closures are
// known, the Send and Sync where-clauses of the called functions (E0277):
// `thread::spawn(move || .. t ..)` needs `T: Send` for a captured T.
// Only type parameters reached through references, tuples, arrays,
// closure captures and a few std containers are judged.
func (fcx *FnCtxt) polyAutoBounds(body *ast.Block) {
	t := fcx.t
	r := fcx.r
	ast.Walk(body, func(n any) bool {
		var id ast.NodeID
		var sp lexer.Span
		switch x := n.(type) {
		case *ast.ItemStmt:
			return false
		case *ast.CallExpr:
			id, sp = x.ID, x.Span
		case *ast.MethodCallExpr:
			id, sp = x.ID, x.Span
		default:
			return true
		}
		c := r.Callees[id]
		if c == nil || c.Kind != CallFn || c.Def == nil || c.Def.Kind != DefFn {
			return true
		}
		g := t.Generics(c.Def)
		if len(c.Args) != len(g.Params) {
			return true
		}
		for _, p := range g.Predicates {
			if p.Trait == nil || p.Maybe || len(p.Trait.Args) == 0 {
				continue
			}
			kind := t.autoKind(p.Trait.Trait)
			if kind == "" {
				continue
			}
			self := Subst(p.Trait.Args[0], c.Args)
			if self.HasInfer() || self.HasError() {
				continue
			}
			if bad, k := fcx.polyNotAuto(kind, self, false, 0, map[autoKey]bool{}); bad != nil {
				what := "sent"
				if k == "Sync" {
					what = "shared"
				}
				fcx.polyClassify("E0277", "`%s` cannot be %s between threads safely", bad, what)
				t.ErrorCode(sp, "E0277", "`%s` cannot be %s between threads safely", bad, what)
				return true
			}
		}
		return true
	})
}

// polyEnvAuto reports whether a where-clause (or a supertrait of one) of
// the body states `ty: kind`.
func (fcx *FnCtxt) polyEnvAuto(kind string, ty Ty) bool {
	for _, e := range fcx.ic.env {
		if len(e.Args) == 0 || e.Args[0] != ty {
			continue
		}
		for _, st := range fcx.t.AllSupertraits(e) {
			if fcx.t.autoKind(st.Trait) == kind {
				return true
			}
		}
	}
	return false
}

// polyNotAuto returns a type parameter that makes ty certainly not
// implement the auto trait kind (Send or Sync), and the auto trait the
// parameter lacks. Through a closure capture, the parameter must lack
// both (how a variable is captured does not matter then).
func (fcx *FnCtxt) polyNotAuto(kind string, ty Ty, captured bool, depth int, seen map[autoKey]bool) (Ty, string) {
	t := fcx.t
	ty = fcx.resolve(ty)
	k := autoKey{kind, ty}
	if depth > 12 || seen[k] || fcx.polyEnvAuto(kind, ty) {
		return nil, ""
	}
	seen[k] = true
	switch ty.Kind {
	case TParam:
		if !isTyParam(ty) {
			return nil, ""
		}
		for _, b := range fcx.paramBounds(ty) {
			if ak := t.autoKind(b.Trait); ak == kind || (captured && ak != "") {
				return nil, ""
			}
		}
		return ty, kind
	case TRef:
		if kind == "Send" && ty.Mut {
			return fcx.polyNotAuto("Send", ty.Elem, captured, depth+1, seen)
		}
		return fcx.polyNotAuto("Sync", ty.Elem, captured, depth+1, seen)
	case TArray, TSlice:
		return fcx.polyNotAuto(kind, ty.Elem, captured, depth+1, seen)
	case TTuple:
		for _, e := range ty.Elems {
			if b, bk := fcx.polyNotAuto(kind, e, captured, depth+1, seen); b != nil {
				return b, bk
			}
		}
	case TClosure:
		ci := t.Closures[ty]
		if ci == nil || !ci.Sealed {
			return nil, ""
		}
		for _, cp := range ci.Captures {
			if cp.Ty == nil || cp.Ty.HasInfer() {
				continue
			}
			ct := cp.Ty
			if cp.ByRef {
				ct = Ref(ct, cp.Mut)
			}
			if b, bk := fcx.polyNotAuto(kind, ct, true, depth+1, seen); b != nil {
				return b, bk
			}
		}
	case TAdt:
		if !t.isStd(ty.Def) {
			return nil, ""
		}
		switch ty.Def.Name {
		case "Box", "Vec", "VecDeque", "Option", "Result", "PhantomData":
			for _, a := range ty.Args {
				if b, bk := fcx.polyNotAuto(kind, a, captured, depth+1, seen); b != nil {
					return b, bk
				}
			}
		}
	}
	return nil, ""
}
