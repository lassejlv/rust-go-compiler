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
//   - E0308: a type parameter against another one or against a type with
//     a known constructor (`u8`, `&T`, `Vec<T>`; no unresolved parts);
//   - E0277: an unsatisfied `T: Trait` where-clause of a called item of a
//     non-std crate, T a type parameter not bounded by Trait, and Trait
//     without impls for all types (`impl<T> Trait for T`).
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
	fcx.checkFn()
	return ps.errs
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
	if (isTyParam(e) && (fcx.rigidTy(a) || (isTyParam(a) && a != e))) || (isTyParam(a) && fcx.rigidTy(e)) {
		fcx.polyClassify("E0308", "mismatched types: expected `%s`, found `%s`", e, a)
		return
	}
	if fcx.unboundProj(e) || fcx.unboundProj(a) {
		fcx.t.poly.next = &polyErr{ok: true} // (consequence)
	}
}

// polyUnsatisfied classifies an unsatisfied where-clause of a called item
// of a non-std crate (E0277): `T: Trait` for a type parameter T whose
// bounds do not mention Trait, where no impl of Trait applies to type
// parameters (only a where-clause could make it hold).
func (fcx *FnCtxt) polyUnsatisfied(o *Obligation, tr *TraitRef) {
	if !fcx.inPoly() || o.From == nil || fcx.t.isStd(o.From) || o.Depth != 0 || len(tr.Args) == 0 {
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
		if impl.Impl.TraitRef.Args[0].Kind == TParam {
			return // a blanket impl
		}
	}
	fcx.polyClassify("E0277", "the trait bound `%s: %s` is not satisfied", self, traitRefDisplay(&TraitRef{Trait: trait, Args: fcx.ic.resolveList(tr.Args)}))
}
