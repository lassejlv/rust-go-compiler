package codegen

// Support for structs with unsized tails (`RcBox<dyn Trait>`,
// `RefCell<[T]>`, ...). The tail field's offset depends on its dynamic
// alignment, computed from the pointer metadata.

import (
	"fmt"

	. "rustgo/compiler/middle"
)

// tailField returns the index and type of the last field of an unsized
// struct type.
func (g *Gen) tailField(t Ty) (int, Ty) {
	fts := g.fieldTypes(t, 0)
	return len(fts) - 1, fts[len(fts)-1]
}

// staticTail reports whether the unsized tail of t is declared as a typed
// flexible array member at a static offset (str / [T]).
func staticTail(tail Ty) bool { return tail.Kind == TStr || tail.Kind == TSlice }

// unsizedAlign is a C expression for the alignment of an unsized value of
// type t with pointer metadata meta.
func (g *Gen) unsizedAlign(t Ty, meta string) string {
	switch t.Kind {
	case TStr:
		return "((uint64_t)1)"
	case TSlice:
		g.defineByValue(t.Elem)
		return fmt.Sprintf("((uint64_t)_Alignof(%s))", g.ctype(t.Elem))
	case TDyn:
		return fmt.Sprintf("((uint64_t)(uintptr_t)((void**)(%s))[2])", meta)
	case TAdt:
		g.defineType(t)
		_, tail := g.tailField(t)
		return fmt.Sprintf("RG_MAXU((uint64_t)_Alignof(%s), %s)", g.ctype(t), g.unsizedAlign(tail, meta))
	}
	return "((uint64_t)1)"
}

// unsizedSize is a C expression for the size of an unsized value.
func (g *Gen) unsizedSize(t Ty, meta string) string {
	switch t.Kind {
	case TStr:
		return fmt.Sprintf("((uint64_t)(%s))", meta)
	case TSlice:
		g.defineByValue(t.Elem)
		return fmt.Sprintf("((uint64_t)(%s) * sizeof(%s))", meta, g.ctype(t.Elem))
	case TDyn:
		return fmt.Sprintf("((uint64_t)(uintptr_t)((void**)(%s))[1])", meta)
	case TAdt:
		_, tail := g.tailField(t)
		return fmt.Sprintf("RG_ALIGN_UP(%s + %s, %s)", g.tailOffset(t, meta), g.unsizedSize(tail, meta), g.unsizedAlign(t, meta))
	}
	return "((uint64_t)0)"
}

// tailOffset is a C expression for the byte offset of the unsized tail
// field of struct t.
func (g *Gen) tailOffset(t Ty, meta string) string {
	g.defineType(t)
	i, tail := g.tailField(t)
	return fmt.Sprintf("RG_ALIGN_UP((uint64_t)__builtin_offsetof(%s, f%d), %s)", g.ctype(t), i, g.unsizedAlign(tail, meta))
}

// unsizeMeta computes the pointer metadata for unsizing src to dst.
func (g *Gen) unsizeMeta(src, dst Ty, oldMeta string) string {
	switch dst.Kind {
	case TSlice:
		if src.Kind == TArray {
			return fmt.Sprintf("%d", g.arrayLen(src))
		}
	case TDyn:
		if src.Kind == TDyn {
			if src.Def == dst.Def {
				return oldMeta
			}
			// supertrait upcast: the source vtable holds the target's
			return fmt.Sprintf("((uint64_t)(uintptr_t)((void**)(%s))[%d])", oldMeta, g.upcastSlot(src, dst))
		}
		return fmt.Sprintf("(uint64_t)(uintptr_t)%s", g.vtable(src, dst))
	case TAdt:
		if src.Kind == TAdt && src.Def == dst.Def {
			p := dst.Def.Adt.TailParam
			return g.unsizeMeta(src.Args[p], dst.Args[p], oldMeta)
		}
	}
	g.fatalf("unsupported unsizing %s -> %s", src, dst)
	return ""
}

// upcastSlot is the vtable slot of src's vtable holding the vtable for
// supertrait object dst.
func (g *Gen) upcastSlot(src, dst Ty) int {
	n := len(g.vtableMethods(src))
	for k, st := range g.supertraitSlots(src) {
		if st == dst.Def {
			return 3 + n + k
		}
	}
	g.fatalf("cannot upcast %s to %s", src, dst)
	return 0
}

// supertraitSlots lists the (strict) supertraits of a dyn type's principal
// trait, in vtable order.
func (g *Gen) supertraitSlots(dyn Ty) []*Def {
	if dyn.Def == nil {
		return nil
	}
	obj := &TraitRef{Trait: dyn.Def, Args: append([]Ty{dyn}, dyn.Args...)}
	var out []*Def
	for _, st := range g.tcx.AllSupertraits(obj) {
		if st.Trait != dyn.Def {
			out = append(out, st.Trait)
		}
	}
	return out
}

// supertraitDyn builds the dyn type for supertrait def of dyn type d.
func (g *Gen) supertraitDyn(d Ty, super *Def) Ty {
	obj := &TraitRef{Trait: d.Def, Args: append([]Ty{d}, d.Args...)}
	for _, st := range g.tcx.AllSupertraits(obj) {
		if st.Trait == super {
			return Dyn(super, st.Args[1:], nil, 0)
		}
	}
	return Dyn(super, nil, nil, 0)
}
