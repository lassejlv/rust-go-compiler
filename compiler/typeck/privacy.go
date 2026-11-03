package typeck

// Privacy checks (mirrors rustc_privacy): items named through module paths
// (E0603), fields (E0616, E0451) and associated functions/methods (E0624)
// must be visible from where they are used.
//
// Paths written by macros (identifiers carrying a macro_rules hygiene mark)
// and anything defined in or used from std (whose visibilities do not
// mirror the real std's) are not checked.

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/resolve"
)

// fromMacro reports whether a path contains identifiers written by a
// macro_rules body.
func fromMacro(p *ast.Path) bool {
	for _, s := range p.Segments {
		if s.Ident.Hyg != 0 {
			return true
		}
	}
	return false
}

// generatedPath reports a path made by the compiler (test and doc-test
// harnesses mark theirs with hygiene marks from 0x7ffffff0 up).
func generatedPath(p *ast.Path) bool {
	for _, s := range p.Segments {
		if s.Ident.Hyg >= 0x7ffffff0 {
			return true
		}
	}
	return false
}

func (t *Tcx) isStd(d *Def) bool {
	return d != nil && d.Crate != nil && t.R.Std != nil && d.Crate == t.R.Std
}

// reportPrivatePath reports E0603 for a path whose segment PrivSeg names an
// item that is not visible.
func (t *Tcx) reportPrivatePath(p *ast.Path, pr resolve.PathResult) {
	if pr.PrivSeg >= len(p.Segments) {
		return
	}
	d := pr.PrivRes.Def
	if d == nil {
		return
	}
	// macro_rules paths are not exempt from privacy, but rustgo's std
	// macros may name std internals, and generated test harnesses name
	// private test functions
	if fromMacro(p) && (t.isStd(d) || generatedPath(p)) {
		return
	}
	kind := d.Kind.String()
	switch {
	case pr.PrivRes.Kind == ResCtor && d.Kind == DefStruct:
		kind = "tuple struct constructor"
		if st, ok := d.Item.Kind.(*ast.StructItem); ok && st.Shape == ast.ShapeUnit {
			kind = "unit struct"
		}
	case d.Kind == DefCrate:
		kind = "crate"
	}
	seg := p.Segments[pr.PrivSeg]
	t.ErrorCode(seg.Ident.Span, "E0603", "%s `%s` is private", kind, seg.Ident.Name)
}

// accessibleFrom reports whether an item with visibility vis declared in
// module owner (of def's crate) is visible from body module from.
func (t *Tcx) accessibleFrom(vis ast.Visibility, owner *Module, ownerDef *Def, from *Module) bool {
	if from == nil || owner == nil {
		return true
	}
	if t.R.Std != nil && (owner.Crate == t.R.Std || from.Crate == t.R.Std) {
		return true
	}
	_ = ownerDef
	return resolve.VisibleFrom(vis, owner, from)
}

// assocItemAccessible reports whether an associated item of an inherent impl
// is visible from module from (trait items are public).
func (t *Tcx) assocItemAccessible(item *Def, from *Module) bool {
	if item == nil || item.Item == nil || !item.InImpl() {
		return true
	}
	impl := item.Parent
	if impl.Impl == nil || impl.Impl.Ast == nil || impl.Impl.Ast.Trait != nil {
		return true
	}
	return t.accessibleFrom(item.Item.Vis, impl.Scope, impl, from)
}

// checkFieldPrivacy reports E0451 for a field named in a struct expression
// or pattern that is not visible here.
func (fcx *FnCtxt) checkFieldPrivacy(name ast.Ident, adt Ty, f *FieldDef, v *VariantDef) {
	adt = fcx.shallow(adt)
	if name.Hyg != 0 || adt.Kind != TAdt || adt.Def.Adt.Kind == AdtEnum || fcx.t.isStd(adt.Def) || fcx.t.isStd(fcx.def) {
		return
	}
	if fcx.t.accessibleFrom(f.Vis, adt.Def.Scope, adt.Def, fcx.curMod()) {
		return
	}
	fcx.t.ErrorCode(name.Span, "E0451", "field `%s` of %s `%s` is private", name.Name, adtKindName(adt.Def), adt.Def.Name)
}

// checkGlobAmbiguity reports E0659 when the first segment of a relative
// path is only found through conflicting glob imports.
func (t *Tcx) checkGlobAmbiguity(p *ast.Path, from *Module, ns Namespace, first resolve.FirstLookup) {
	if p.Global || len(p.Segments) == 0 || fromMacro(p) || from == nil || (t.R.Std != nil && from.Crate == t.R.Std) {
		return
	}
	name := p.Segments[0].Ident.Name
	switch name {
	case "self", "super", "crate", "Self":
		return
	}
	if len(p.Segments) > 1 {
		ns = TypeNS
	}
	if first != nil {
		if _, ok := first(name, ns); ok {
			return
		}
	}
	if t.R.GlobAmbiguous(from, name, ns) {
		t.ErrorCode(p.Segments[0].Ident.Span, "E0659", "`%s` is ambiguous", name)
	}
}
