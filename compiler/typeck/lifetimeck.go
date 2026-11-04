package typeck

// Missing lifetime specifiers (rustc E0106): references in struct/enum
// fields need a named lifetime, and an elided lifetime in a function's
// return type needs exactly one lifetime in the inputs (or a `&self`
// receiver). The check is syntactic; whenever the inputs may hide
// lifetimes it cannot count (e.g. `impl Trait`, unresolvable paths) it
// stays silent.

import (
	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

func (t *Tcx) checkLifetimes() {
	for _, d := range t.R.Defs {
		if t.isStd(d) || d.Item == nil {
			continue
		}
		switch d.Kind {
		case DefStruct, DefUnion, DefEnum:
			var fields []*ast.FieldDef
			switch k := d.Item.Kind.(type) {
			case *ast.StructItem:
				fields = k.Fields
			case *ast.UnionItem:
				fields = k.Fields
			case *ast.EnumItem:
				for _, v := range k.Variants {
					fields = append(fields, v.Fields...)
				}
			}
			for _, f := range fields {
				if r := elidedRef(f.Ty); r != nil {
					t.ErrorCode(r.Span, "E0106", "missing lifetime specifier")
				}
			}
		case DefTyAlias:
			if ta, ok := d.Item.Kind.(*ast.TypeAliasItem); ok && ta.Ty != nil && !d.IsAssoc() {
				if r := elidedRef(ta.Ty); r != nil {
					t.ErrorCode(r.Span, "E0106", "missing lifetime specifier")
				}
			}
		case DefFn:
			if d.Fn == nil || d.Fn.Ast == nil || d.Fn.Ast.Decl.Output == nil {
				continue
			}
			r := elidedRef(d.Fn.Ast.Decl.Output)
			if r == nil {
				continue
			}
			n, certain := t.inputLifetimes(d)
			if n >= 2 || (n == 0 && certain) {
				t.ErrorCode(r.Span, "E0106", "missing lifetime specifier")
			}
		}
	}
}

// elidedRef finds a reference type without a lifetime that does not live
// in its own elision scope (fn pointers, Fn(..) sugar, trait object and
// impl Trait bounds).
func elidedRef(ty ast.Type) *ast.RefType {
	var found *ast.RefType
	ast.Walk(ty, func(n any) bool {
		if found != nil {
			return false
		}
		switch x := n.(type) {
		case *ast.RefType:
			if x.Lifetime == "" {
				found = x
				return false
			}
		case *ast.FnPtrType, *ast.DynTraitType, *ast.ImplTraitType, *ast.MacroType:
			return false
		case *ast.Path:
			for _, sg := range x.Segments {
				if sg.Args != nil && sg.Args.Parenthesized {
					return false
				}
			}
		}
		return true
	})
	return found
}

// inputLifetimes counts the distinct lifetimes of a function's inputs
// available for elision in its return type. certain is false when some
// input may hide lifetimes the count does not see.
func (t *Tcx) inputLifetimes(d *Def) (n int, certain bool) {
	decl := d.Fn.Ast.Decl
	if sp := decl.SelfParam; sp != nil {
		switch sp.Kind {
		case ast.SelfRef, ast.SelfRefMut:
			return 1, true // the receiver's lifetime is used
		case ast.SelfTyped:
			return 1, false
		}
	}
	certain = true
	named := map[string]bool{}
	tyParams := map[string]bool{}
	for g := d; g != nil; g = g.Parent {
		if ga := genericsAst(g); ga != nil {
			for _, p := range ga.Params {
				if p.Kind != ast.GPLifetime {
					tyParams[p.Name.Name] = true
				}
			}
		}
		if g.Kind == DefMod || g.Kind == DefCrate {
			break
		}
	}
	addLt := func(lt string) {
		if lt == "" || lt == "'_" {
			n++
		} else if !named[lt] {
			named[lt] = true
			n++
		}
	}
	for _, p := range decl.Params {
		ast.Walk(p.Ty, func(x any) bool {
			switch y := x.(type) {
			case *ast.RefType:
				addLt(y.Lifetime)
			case *ast.FnPtrType, *ast.DynTraitType:
				return false
			case *ast.ImplTraitType, *ast.MacroType:
				certain = false
				return false
			case *ast.PathType:
				if y.QSelf != nil {
					certain = false
					return true
				}
				segs := y.Path.Segments
				if len(segs) == 0 {
					return true
				}
				hasLtArg := false
				for _, s := range segs {
					if s.Args != nil && s.Args.Parenthesized {
						return false
					}
					if s.Args != nil {
						for _, a := range s.Args.Args {
							if a.Lifetime != "" {
								addLt(a.Lifetime)
								hasLtArg = true
							}
						}
					}
				}
				first := segs[0].Ident.Name
				if !y.Path.Global && len(segs) == 1 && (tyParams[first] || resolveIsPrim(first)) {
					return true
				}
				if first == "Self" {
					certain = false
					return true
				}
				if hasLtArg {
					return true
				}
				// a path may hide lifetime parameters
				names := make([]string, len(segs))
				for i, s := range segs {
					names[i] = s.Ident.Name
				}
				if !y.Path.Global && tyParams[first] {
					certain = false // T::Assoc
					return true
				}
				pr, err := t.R.ResolvePath(d.Scope, names, y.Path.Global, TypeNS, nil)
				if err != nil || pr.Consumed != len(segs) || pr.Res.Kind != ResDef || pr.Res.Def == nil {
					if err == nil && pr.Res.Kind == ResPrim {
						return true
					}
					certain = false
					return true
				}
				ga := genericsAst(pr.Res.Def)
				switch pr.Res.Def.Kind {
				case DefStruct, DefEnum, DefUnion, DefTyAlias:
				default:
					certain = false
					return true
				}
				if ga != nil {
					for _, gp := range ga.Params {
						if gp.Kind == ast.GPLifetime {
							addLt("")
						}
					}
				}
			}
			return true
		})
	}
	return n, certain
}

func resolveIsPrim(name string) bool {
	switch name {
	case "bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32", "f64":
		return true
	}
	return false
}
