package codegen

import (
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
)

// rustcTypeName renders a type the way rustc's `core::any::type_name`
// does: fully qualified paths, with the std library's items placed in the
// real `core` / `alloc` / `std` crates (rustgo has a single std crate).
// Lifetimes are not tracked, so they are omitted.
func rustcTypeName(t Ty) string {
	var b strings.Builder
	writeTypeName(&b, t)
	return b.String()
}

// stdTypePaths overrides the paths of std items whose real location differs
// from rustgo's module layout (keyed by rustgo module path).
var stdTypePaths = map[string]string{
	"collections::hash_map::HashMap":       "std::collections::hash::map::HashMap",
	"collections::hash_set::HashSet":       "std::collections::hash::set::HashSet",
	"collections::btree_map::BTreeMap":     "alloc::collections::btree::map::BTreeMap",
	"collections::btree_set::BTreeSet":     "alloc::collections::btree::set::BTreeSet",
	"collections::vec_deque::VecDeque":     "alloc::collections::vec_deque::VecDeque",
	"collections::binary_heap::BinaryHeap": "alloc::collections::binary_heap::BinaryHeap",
	"collections::linked_list::LinkedList": "alloc::collections::linked_list::LinkedList",
	"sync::mutex::Mutex":                   "std::sync::poison::mutex::Mutex",
	"sync::rwlock::RwLock":                 "std::sync::poison::rwlock::RwLock",
	"sync::arc::Arc":                       "alloc::sync::Arc",
	"sync::arc::Weak":                      "alloc::sync::Weak",
	"time::Instant":                        "std::time::Instant",
	"time::SystemTime":                     "std::time::SystemTime",
	"vec::into_iter::IntoIter":             "alloc::vec::into_iter::IntoIter",
	"ffi::OsString":                        "std::ffi::os_str::OsString",
	"ffi::OsStr":                           "std::ffi::os_str::OsStr",
	"io::error::Error":                     "core::io::error::Error",
	"hash::RandomState":                    "std::hash::random::RandomState",
	"hash::DefaultHasher":                  "std::hash::random::DefaultHasher",
}

// stdCrateOf maps rustgo's std top-level modules to the real crate.
var stdCrateOf = map[string]string{
	"vec": "alloc", "string": "alloc", "boxed": "alloc", "rc": "alloc", "borrow": "alloc",
	"collections": "alloc",
	"option":      "core", "result": "core", "cell": "core", "cmp": "core", "clone": "core",
	"marker": "core", "ops": "core", "iter": "core", "num": "core", "char": "core",
	"str": "core", "slice": "core", "array": "core", "mem": "core", "ptr": "core",
	"fmt": "core", "hash": "core", "any": "core", "convert": "core", "default": "core",
	"time": "core", "pin": "core", "future": "core", "task": "core", "ascii": "core",
	"hint": "core", "intrinsics": "core", "arch": "core", "tuple": "core",
	"f32": "core", "f64": "core", "panic": "core", "error": "core",
}

func defModulePath(d *Def) ([]string, *Def) {
	var parts []string
	var crate *Def
	for x := d; x != nil; x = x.Parent {
		if x.Kind == DefCrate {
			crate = x
			break
		}
		if x.Kind == DefImpl || x.Name == "" {
			continue
		}
		parts = append(parts, x.Name)
	}
	for i, j := 0, len(parts)-1; i < j; i, j = i+1, j-1 {
		parts[i], parts[j] = parts[j], parts[i]
	}
	return parts, crate
}

func defTypeNamePath(d *Def) string {
	parts, crate := defModulePath(d)
	crateName := ""
	if crate != nil {
		crateName = crate.Name
	} else if d.Crate != nil {
		crateName = d.Crate.Name
	}
	if crateName == "std" && len(parts) > 0 {
		key := strings.Join(parts, "::")
		if p, ok := stdTypePaths[key]; ok {
			return p
		}
		real := "std"
		if c, ok := stdCrateOf[parts[0]]; ok {
			real = c
		}
		// sync::atomic lives in core; the rest of sync in std/alloc.
		if parts[0] == "sync" && len(parts) > 1 && parts[1] == "atomic" {
			real = "core"
		}
		return real + "::" + key
	}
	if crateName == "" {
		return strings.Join(parts, "::")
	}
	if len(parts) == 0 {
		return crateName
	}
	return crateName + "::" + strings.Join(parts, "::")
}

func writeTypeNameList(b *strings.Builder, ts []Ty) {
	for i, x := range ts {
		if i > 0 {
			b.WriteString(", ")
		}
		writeTypeName(b, x)
	}
}

func writeTypeName(b *strings.Builder, t Ty) {
	switch t.Kind {
	case TTuple:
		b.WriteByte('(')
		writeTypeNameList(b, t.Elems)
		if len(t.Elems) == 1 {
			b.WriteByte(',')
		}
		b.WriteByte(')')
	case TArray:
		b.WriteByte('[')
		writeTypeName(b, t.Elem)
		b.WriteString("; ")
		if t.Len.Kind == TConst {
			b.WriteString(strconv.FormatUint(t.Len.Val, 10))
		} else {
			b.WriteString(t.Len.String())
		}
		b.WriteByte(']')
	case TSlice:
		b.WriteByte('[')
		writeTypeName(b, t.Elem)
		b.WriteByte(']')
	case TRef:
		b.WriteByte('&')
		if t.Mut {
			b.WriteString("mut ")
		}
		writeTypeName(b, t.Elem)
	case TPtr:
		if t.Mut {
			b.WriteString("*mut ")
		} else {
			b.WriteString("*const ")
		}
		writeTypeName(b, t.Elem)
	case TAdt:
		path := defTypeNamePath(t.Def)
		args := t.Args
		// Newer std: AtomicUsize etc. are aliases of the generic Atomic<T>.
		if strings.HasPrefix(path, "core::sync::atomic::Atomic") && len(args) == 0 {
			if p := strings.TrimPrefix(path, "core::sync::atomic::Atomic"); p != "" && p != "Bool" && p != "Ptr" {
				b.WriteString("core::sync::atomic::Atomic<" + strings.ToLower(p) + ">")
				return
			}
		}
		b.WriteString(path)
		lts, defaults := adtGenericsInfo(t.Def)
		// Trailing type args equal to their declared defaults are elided.
		for len(args) > 0 && len(defaults) >= len(args) {
			d := defaults[len(args)-1]
			a := args[len(args)-1]
			if d == "" || a.Kind != TAdt || len(a.Args) != 0 || a.Def.Name != d {
				break
			}
			args = args[:len(args)-1]
		}
		if lts > 0 || len(args) > 0 {
			b.WriteByte('<')
			for i := 0; i < lts; i++ {
				if i > 0 {
					b.WriteString(", ")
				}
				b.WriteString("'_")
			}
			if lts > 0 && len(args) > 0 {
				b.WriteString(", ")
			}
			writeTypeNameList(b, args)
			b.WriteByte('>')
		}
	case TFnDef:
		b.WriteString(defTypeNamePath(t.Def))
		if len(t.Args) > 0 {
			b.WriteString("<")
			writeTypeNameList(b, t.Args)
			b.WriteByte('>')
		}
	case TFnPtr:
		b.WriteString("fn(")
		writeTypeNameList(b, t.Elems)
		b.WriteByte(')')
		if !t.Ret.IsUnit() {
			b.WriteString(" -> ")
			writeTypeName(b, t.Ret)
		}
	case TClosure:
		if t.Def != nil {
			b.WriteString(defTypeNamePath(t.Def))
			b.WriteString("::")
		}
		b.WriteString("{{closure}}")
	case TDyn:
		b.WriteString("dyn ")
		if t.Def != nil {
			b.WriteString(defTypeNamePath(t.Def))
			name := t.Def.Name
			isFn := (name == "Fn" || name == "FnMut" || name == "FnOnce") && len(t.Args) == 1 && t.Args[0].Kind == TTuple
			if isFn {
				b.WriteByte('(')
				writeTypeNameList(b, t.Args[0].Elems)
				b.WriteByte(')')
				for _, bd := range t.Bindings {
					if bd.Name == "Output" && !bd.Ty.IsUnit() {
						b.WriteString(" -> ")
						writeTypeName(b, bd.Ty)
					}
				}
			} else if len(t.Args) > 0 || len(t.Bindings) > 0 {
				b.WriteByte('<')
				writeTypeNameList(b, t.Args)
				for i, bd := range t.Bindings {
					if i > 0 || len(t.Args) > 0 {
						b.WriteString(", ")
					}
					b.WriteString(bd.Name)
					b.WriteString(" = ")
					writeTypeName(b, bd.Ty)
				}
				b.WriteByte('>')
			}
		}
		if t.Sub&DynSend != 0 {
			b.WriteString(" + core::marker::Send")
		}
		if t.Sub&DynSync != 0 {
			b.WriteString(" + core::marker::Sync")
		}
		if t.Sub&DynUnpin != 0 {
			b.WriteString(" + core::marker::Unpin")
		}
	default:
		b.WriteString(t.String())
	}
}

// adtGenericsInfo returns the number of lifetime params of an ADT and, for
// each type/const param in order, the last path segment of its default type
// ("" when it has none).
func adtGenericsInfo(d *Def) (int, []string) {
	if d == nil || d.Item == nil {
		return 0, nil
	}
	var g *ast.Generics
	switch k := d.Item.Kind.(type) {
	case *ast.StructItem:
		g = k.Generics
	case *ast.EnumItem:
		g = k.Generics
	case *ast.UnionItem:
		g = k.Generics
	}
	if g == nil {
		return 0, nil
	}
	lts := 0
	var defaults []string
	for _, p := range g.Params {
		if p.Kind == ast.GPLifetime {
			lts++
			continue
		}
		name := ""
		if p.Default != nil {
			name = ast.TypeString(p.Default)
			if i := strings.LastIndex(name, "::"); i >= 0 {
				name = name[i+2:]
			}
		}
		defaults = append(defaults, name)
	}
	return lts, defaults
}
