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
	R *resolve.Resolver

	Errors    []string
	errSeen   map[string]bool
	MaxErrors int

	traitImpls   map[*Def][]*Def
	inherent     map[string][]*Def // simplified self type -> inherent impls
	traitsByItem map[string][]*Def // assoc item name -> traits declaring it
	implsBuilt   bool
	implsIndexed bool // BuildImpls finished (traitImpls/inherent complete)

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
	msg := fmt.Sprintf("%s: error: %s", sp.Lo, fmt.Sprintf(format, args...))
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
				key += fmt.Sprintf("%p,", a) // types are interned
			}
			if prev := seen[key]; prev != nil {
				t.Errorf(impl.Span, "conflicting implementations of trait `%s` for type `%s` (first at %s)", tr.Name, impl.Impl.TraitRef.Args[0], prev.Span.Lo)
				continue
			}
			seen[key] = impl
		}
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
