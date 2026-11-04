package typeck

import . "rustgo/compiler/middle"

// nonConstStd lists std functions that are not `const` in rustc (they
// allocate): calling them in a constant context is E0015. rustgo's std
// does not mark every function const that std does, so calls of other
// non-const std functions are not reported.
var nonConstStd = map[string]bool{
	"Box::new": true, "Box::pin": true, "Rc::new": true, "Arc::new": true,
	"HashMap::new": true, "HashMap::with_capacity": true,
	"HashSet::new": true, "HashSet::with_capacity": true,
	"Vec::with_capacity": true, "String::with_capacity": true,
	"VecDeque::with_capacity": true,
	"vec::from_elem": true, "vec::from_array": true,
	"fmt::format": true, "fmt::__format": true,
}

// stdFnName names a std function by its impl's self type or module and
// its name (`Box::new`, `vec::from_elem`).
func stdFnName(d *Def) string {
	if d == nil || d.Parent == nil {
		return ""
	}
	p := d.Parent
	switch p.Kind {
	case DefImpl:
		if p.Impl == nil || p.Impl.TraitRef != nil || p.Impl.SelfTy == nil || p.Impl.SelfTy.Kind != TAdt || p.Impl.SelfTy.Def == nil {
			return ""
		}
		return p.Impl.SelfTy.Def.Name + "::" + d.Name
	case DefMod:
		return p.Name + "::" + d.Name
	}
	return ""
}

func nonConstStdFn(d *Def) bool {
	if d == nil || d.Kind != DefFn || d.Crate == nil || d.Crate.Name != "std" {
		return false
	}
	return nonConstStd[stdFnName(d)]
}
