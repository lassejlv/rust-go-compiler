// Package typeck performs type collection, inference, trait selection and
// method resolution (mirrors rustc_hir_analysis + rustc_hir_typeck +
// rustc_trait_selection).
//
// Generic items are checked per instantiation ("templates"): a function
// body is type checked with its generic parameters substituted by concrete
// types. This makes trait method resolution and associated-type
// normalization operate on concrete types only.
package typeck

import (
	"fmt"
	"os"
	"sort"
	"strings"

	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	. "rustgo/compiler/middle"
	"rustgo/compiler/resolve"
)

// Tcx is the global typing context.
type Tcx struct {
	// PolyClean: the generic functions of local crates whose polymorphic
	// check found no error (see CheckGenericBodies)
	PolyClean []PolyBody
	// pendingBindings: associated type bindings of trait bounds
	// (`T: Trait<Name = X>`) in local crates, checked by checkBindingNames
	pendingBindings []pendingBinding
	// polyResults: the results of the last polymorphic body check, when
	// it found no error (see CheckGenericBodies)
	polyResults *Results
	autoTraits  map[string]*Def // std's Send and Sync (see autotrait.go)
	R           *resolve.Resolver

	layouts *layoutCaches // layout.go

	Errors    []string
	errSeen   map[string]bool
	MaxErrors int

	traitImpls   map[*Def][]*Def
	inherent     map[string][]*Def // simplified self type -> inherent impls
	traitsByItem map[string][]*Def // assoc item name -> traits declaring it
	implsBuilt   bool
	implsIndexed bool // BuildImpls finished (traitImpls/inherent complete)

	// Caches for fully concrete selections / projections (traits.go).
	selCache   map[string]Selection
	selActive  map[string]bool
	projCache  map[string]Ty
	projActive map[string]bool

	// quiet > 0 suppresses error reporting (speculative generic checks).
	quiet     int
	quietErrs int
	// poly is the polymorphic check of a generic body in progress (genbody.go).
	poly *polyState
	// methodHints caches, per generic fn, the trait of each trait-method
	// call resolved in the generic body (see methodTraitHints).
	methodHints map[*Def]map[ast.NodeID]*Def
	// methodHintArgs: the trait args (Self first) of those calls in the
	// generic body
	methodHintArgs map[*Def]map[ast.NodeID][]Ty
	// genPicks caches, per generic fn, the method calls of its generic
	// body as resolved against its bounds (see genericPicks)
	genPicks map[*Def]map[ast.NodeID]*genPick
	// genPickOuter: the error mode outside each running genericPicks check
	genPickOuter []genPickOuter

	paramOf   map[*Def]*GenericParamDef
	synthetic map[ast.NodeID]int // impl-Trait-in-arg-position type node -> param index

	results    map[string]*Results
	inProgress map[string]bool
	// OpaqueHidden maps an Opaque type (fn, concrete args, idx) to its hidden type.
	OpaqueHidden map[Ty]Ty
	// Closures maps closure types to their info.
	Closures map[Ty]*ClosureInfo

	constCache map[*Def]*ConstValue
}

// NewTcx creates a typing context over resolved crates.
func NewTcx(r *resolve.Resolver) *Tcx {
	t := &Tcx{
		R:            r,
		errSeen:      map[string]bool{},
		MaxErrors:    30,
		traitImpls:   map[*Def][]*Def{},
		inherent:     map[string][]*Def{},
		traitsByItem: map[string][]*Def{},
		paramOf:      map[*Def]*GenericParamDef{},
		synthetic:    map[ast.NodeID]int{},
		results:      map[string]*Results{},
		selCache:     map[string]Selection{},
		selActive:    map[string]bool{},
		projCache:    map[string]Ty{},
		projActive:   map[string]bool{},
		inProgress:   map[string]bool{},
		OpaqueHidden: map[Ty]Ty{},
		Closures:     map[Ty]*ClosureInfo{},
		constCache:   map[*Def]*ConstValue{},
	}
	return t
}

type fatalError struct{}

// Errorf records a diagnostic at span sp.
func (t *Tcx) Errorf(sp lexer.Span, format string, args ...any) {
	if t.quiet > 0 {
		t.quietErrs++
		if t.poly != nil {
			t.polyRecord(sp, fmt.Sprintf(format, args...))
		}
		if os.Getenv("RUSTGO_TRACE_HINTS") != "" {
			fmt.Fprintf(os.Stderr, "  (quiet) %s: %s\n", sp.Lo, fmt.Sprintf(format, args...))
		}
		return
	}
	text := fmt.Sprintf(format, args...)
	msg := fmt.Sprintf("%s: error: %s", sp.Lo, text)
	if strings.HasPrefix(text, "\x00") {
		// ErrorCode: "\x00E0XXX\x00message"
		if i := strings.IndexByte(text[1:], 0); i >= 0 {
			msg = fmt.Sprintf("%s: error[%s]: %s", sp.Lo, text[1:1+i], text[2+i:])
		}
	}
	if t.errSeen[msg] {
		return
	}
	t.errSeen[msg] = true
	t.Errors = append(t.Errors, msg)
	if os.Getenv("RUSTGO_PANIC_ON_ERROR") != "" {
		panic(msg)
	}
	if len(t.Errors) >= t.MaxErrors {
		panic(fatalError{})
	}
}

// ErrorCode records a diagnostic with a rustc error code (E0XXX).
func (t *Tcx) ErrorCode(sp lexer.Span, code string, format string, args ...any) {
	t.Errorf(sp, "\x00"+code+"\x00"+format, args...)
}

// Err returns the accumulated errors (nil if none).
func (t *Tcx) Err() error {
	if len(t.Errors) == 0 {
		return nil
	}
	return fmt.Errorf("%s", strings.Join(t.Errors, "\n"))
}

// Guard runs f, converting fatal-error panics into a normal return.
func (t *Tcx) Guard(f func()) (aborted bool) {
	defer func() {
		if r := recover(); r != nil {
			if _, ok := r.(fatalError); ok {
				aborted = true
				return
			}
			panic(r)
		}
	}()
	f()
	return false
}

// Lang returns a lang item (panics with a clear message if missing).
func (t *Tcx) Lang(name string) *Def {
	d := t.R.LangItems[name]
	if d == nil {
		panic("missing lang item: " + name)
	}
	return d
}

// LangOpt returns a lang item or nil.
func (t *Tcx) LangOpt(name string) *Def { return t.R.LangItems[name] }

// ---------------------------------------------------------------------------
// Impl indexing

// SimplifyKey returns a coarse key for a self type used to index inherent
// impls ("" for types that cannot be indexed, e.g. params / infer vars).
func SimplifyKey(t Ty) string {
	switch t.Kind {
	case TAdt:
		return fmt.Sprintf("adt%d", t.Def.ID)
	case TBool, TChar, TInt, TFloat, TStr, TNever:
		return t.Key()
	case TSlice:
		return "slice"
	case TArray:
		return "array"
	case TRef:
		if t.Mut {
			return "refmut"
		}
		return "ref"
	case TPtr:
		if t.Mut {
			return "ptrmut"
		}
		return "ptr"
	case TTuple:
		return fmt.Sprintf("tuple%d", len(t.Elems))
	case TDyn:
		return fmt.Sprintf("dyn%d", t.Def.ID)
	case TFnPtr:
		return "fnptr"
	case TClosure:
		return "closure"
	case TFnDef:
		return "fndef"
	}
	return ""
}

// BuildImpls lowers all impl headers and indexes them.
func (t *Tcx) BuildImpls() {
	if t.implsBuilt {
		return
	}
	t.implsBuilt = true
	for _, tr := range t.R.Traits {
		for name := range tr.Trait.Items {
			t.traitsByItem[name] = append(t.traitsByItem[name], tr)
		}
	}
	for _, d := range t.R.Defs {
		if d.Kind == DefStruct {
			t.AdtFields(d)
		}
	}
	for _, impl := range t.R.Impls {
		t.ImplHeader(impl)
		if impl.Impl.TraitRef != nil {
			tr := impl.Impl.TraitRef.Trait
			t.traitImpls[tr] = append(t.traitImpls[tr], impl)
		} else {
			k := SimplifyKey(impl.Impl.SelfTy)
			t.inherent[k] = append(t.inherent[k], impl)
		}
	}
	for _, tr := range t.R.Traits {
		sort.SliceStable(t.traitImpls[tr], func(i, j int) bool { return t.traitImpls[tr][i].ID < t.traitImpls[tr][j].ID })
	}
	t.implsIndexed = true
	t.checkCoherence()
}

// checkCoherence reports trait impls without generic parameters that
// implement the same trait for the same type twice (rustc E0119).
func (t *Tcx) checkCoherence() {
	for _, tr := range t.R.Traits {
		seen := map[string]*Def{}
		for _, impl := range t.traitImpls[tr] {
			if impl.Impl.Negative || len(t.Generics(impl).Params) > 0 {
				continue
			}
			key := ""
			for _, a := range impl.Impl.TraitRef.Args {
				key += fmt.Sprintf("%d,", a.ID()) // types are interned
			}

			if prev := seen[key]; prev != nil {
				t.ErrorCode(impl.Span, "E0119", "conflicting implementations of trait `%s` for type `%s`", tr.Name, impl.Impl.TraitRef.Args[0])
				continue
			}
			seen[key] = impl
		}
		// a generic impl without bounds overlaps every impl it covers
		if t.isStd(tr) {
			if tr.LangItem == "from" {
				t.checkReflexiveFrom(tr)
			}
			continue
		}
		for _, a := range t.traitImpls[tr] {
			if a.Impl.Negative || len(t.Generics(a).Params) == 0 || t.isStd(a) {
				continue
			}
			for _, b := range t.traitImpls[tr] {
				if b == a || b.Impl.Negative || t.isStd(b) || (len(t.Generics(b).Params) > 0 && b.ID < a.ID) {
					continue
				}
				if t.blanketCovers(a, b) || (len(t.Generics(b).Params) > 0 && t.genericImplsOverlap(a, b)) {
					later := b
					if a.ID > b.ID {
						later = a
					}
					t.ErrorCode(later.Span, "E0119", "conflicting implementations of trait `%s` for type `%s`", tr.Name, b.Impl.TraitRef.Args[0])
				}
			}
		}
	}
	t.checkImpls()
}

// checkReflexiveFrom reports impls of `From<X> for Y` outside std that
// overlap core's `impl<T> From<T> for T` (E0119): X and Y unify (the
// impl's parameters taken as unknowns) and none of the impl's
// where-clauses certainly fails then.
func (t *Tcx) checkReflexiveFrom(from *Def) {
	for _, impl := range t.traitImpls[from] {
		if t.isStd(impl) || impl.Impl.Negative || impl.Impl.Ast == nil {
			continue
		}
		tr := impl.Impl.TraitRef
		if len(tr.Args) != 2 || tr.Args[0].HasProj() || tr.Args[1].HasProj() || tr.Args[0].HasError() || tr.Args[1].HasError() ||
			tr.Args[0].HasOpaque() || tr.Args[1].HasOpaque() {
			continue
		}
		g := t.Generics(impl)
		konst := false
		for _, p := range g.Params {
			konst = konst || p.Const
		}
		if konst {
			continue
		}
		ic := newInferCtxt(t)
		args := make([]Ty, len(g.Params))
		for i := range args {
			args[i] = ic.newVar()
		}
		a, b := Subst(tr.Args[0], args), Subst(tr.Args[1], args)
		if !ic.unify(a, b) {
			continue
		}
		fails := false
		for _, p := range g.Predicates {
			if p.Maybe || p.Trait == nil {
				continue
			}
			if ic.evaluate(p.Subst(args), 0) == SelNone {
				fails = true
			}
		}
		if fails {
			continue
		}
		t.ErrorCode(impl.Span, "E0119", "conflicting implementations of trait `From<%s>` for type `%s`", tr.Args[1], tr.Args[0])
	}
}

// TraitImpls returns all impls of a trait.
func (t *Tcx) TraitImpls(tr *Def) []*Def { return t.traitImpls[tr] }

// InherentImpls returns inherent impls whose self type has the given key.
func (t *Tcx) InherentImpls(key string) []*Def { return t.inherent[key] }

// ---------------------------------------------------------------------------
// Instances

// Instance is a function (or const/static/closure body owner) with concrete
// generic arguments.
type Instance struct {
	Def  *Def
	Args []Ty
}

func InstKey(d *Def, args []Ty) string {
	var b strings.Builder
	fmt.Fprintf(&b, "%d<", d.ID)
	for i, a := range args {
		if i > 0 {
			b.WriteByte(',')
		}
		b.WriteString(a.Key())
	}
	b.WriteByte('>')
	return b.String()
}

func (i Instance) Key() string { return InstKey(i.Def, i.Args) }

func (i Instance) String() string {
	var b strings.Builder
	b.WriteString(i.Def.Path())
	if len(i.Args) > 0 {
		b.WriteString("::<")
		for j, a := range i.Args {
			if j > 0 {
				b.WriteString(", ")
			}
			b.WriteString(a.String())
		}
		b.WriteString(">")
	}
	return b.String()
}

// methodTraitHints type-checks def generically (its own parameters as
// types) and returns, for each method call resolved to a trait method, that
// trait. Instances use it where the substituted receiver type implements
// several traits with the method (e.g. serde's `S::SerializeMap` and
// `S::SerializeSeq` both become one concrete type): the generic body
// knows which trait was meant. Errors of the speculative check are ignored.
func (t *Tcx) methodTraitHints(def *Def) map[ast.NodeID]*Def {
	if t.methodHints == nil {
		t.methodHints = map[*Def]map[ast.NodeID]*Def{}
	}
	if h, ok := t.methodHints[def]; ok {
		return h
	}
	t.methodHints[def] = nil // recursion guard
	if def.Kind != DefFn || def.Fn == nil || def.Fn.Ast == nil || def.Fn.Ast.Body == nil {
		return nil
	}
	args := t.Generics(def).Identity()
	if len(args) == 0 {
		return nil
	}
	hints := map[ast.NodeID]*Def{}
	hintArgs := map[ast.NodeID][]Ty{}
	if t.methodHintArgs == nil {
		t.methodHintArgs = map[*Def]map[ast.NodeID][]Ty{}
	}
	t.methodHintArgs[def] = hintArgs
	t.quiet++
	func() {
		defer func() { recover() }()
		fcx := t.newFnCtxt(def, args)
		fcx.checkFn()
		for id, c := range fcx.r.Callees {
			if c != nil && c.Def != nil && c.Def.InTrait() && c.Def.Parent != nil {
				hints[id] = c.Def.Parent
				if n := len(t.Generics(c.Def.Parent).Params); len(c.Args) >= n {
					hintArgs[id] = fcx.ic.resolveList(c.Args[:n])
				}
			}
		}
	}()
	t.quiet--
	if os.Getenv("RUSTGO_TRACE_HINTS") != "" {
		fmt.Fprintf(os.Stderr, "hints for %s: %d calls, %d quiet errors\n", def.Path(), len(hints), t.quietErrs)
	}
	t.methodHints[def] = hints
	return hints
}
