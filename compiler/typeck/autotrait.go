package typeck

// Auto traits Send and Sync (rustc_trait_selection's auto trait
// candidates): a type implements them when all its components do, unless
// an explicit (positive or negative) impl decides. Raw pointers implement
// neither; `&T: Send` needs `T: Sync`. Types of the embedded std are judged
// by their type arguments, with the exceptions the real std declares
// (Rc, Cell, Mutex, Arc, ...), since rustgo's std does not mirror the real
// std's internals. Unknown parts (inference variables, type parameters,
// projections, opaque types, closures whose captures are not known yet)
// are taken to implement them; closures are checked again after capture
// analysis (checkCallBounds). Other auto traits always hold.

import (
	. "rustgo/compiler/middle"
)

// autoKind returns "Send"/"Sync" for std's Send and Sync traits, "" otherwise.
func (t *Tcx) autoKind(trait *Def) string {
	if trait != nil && trait.Trait != nil && trait.Trait.Auto && t.isStd(trait) && (trait.Name == "Send" || trait.Name == "Sync") {
		return trait.Name
	}
	return ""
}

// autoHolds reports whether ty implements Send or Sync (kind).
func (ic *InferCtxt) autoHolds(kind string, ty Ty, depth int, seen map[autoKey]bool) bool {
	if depth > 40 || ty == nil {
		return true
	}
	ty = ic.shallow(ty)
	k := autoKey{kind, ty}
	if seen[k] {
		return true // coinductive
	}
	seen[k] = true
	t := ic.t
	switch ty.Kind {
	case TPtr:
		return false
	case TRef:
		if kind == "Send" && ty.Mut {
			return ic.autoHolds("Send", ty.Elem, depth+1, seen)
		}
		return ic.autoHolds("Sync", ty.Elem, depth+1, seen)
	case TArray, TSlice:
		return ic.autoHolds(kind, ty.Elem, depth+1, seen)
	case TTuple:
		for _, e := range ty.Elems {
			if !ic.autoHolds(kind, e, depth+1, seen) {
				return false
			}
		}
		return true
	case TDyn:
		bit := DynSend
		if kind == "Sync" {
			bit = DynSync
		}
		if ty.Sub&bit != 0 || ty.Def == nil {
			return true
		}
		obj := &TraitRef{Trait: ty.Def, Args: append([]Ty{ty}, ty.Args...)}
		for _, st := range t.AllSupertraits(obj) {
			if t.autoKind(st.Trait) == kind {
				return true
			}
		}
		return false
	case TClosure:
		ci := t.Closures[ty]
		if ci == nil || !t.sealedClosure(ty) {
			return true
		}
		for _, cp := range ci.Captures {
			if cp.Ty == nil || cp.Ty.HasInfer() {
				continue
			}
			ct := cp.Ty
			if cp.ByRef {
				ct = Ref(ct, cp.Mut)
			}
			if !ic.autoHolds(kind, ct, depth+1, seen) {
				return false
			}
		}
		return true
	case TAdt:
		return ic.adtAutoHolds(kind, ty, depth, seen)
	}
	return true
}

type autoKey struct {
	kind string
	ty   Ty
}

func (ic *InferCtxt) adtAutoHolds(kind string, ty Ty, depth int, seen map[autoKey]bool) bool {
	t := ic.t
	d := ty.Def
	// explicit impls decide
	if trait := t.stdAutoTrait(kind); trait != nil {
		for _, impl := range t.traitImpls[trait] {
			ii := impl.Impl
			if ii == nil || ii.TraitRef == nil || len(ii.TraitRef.Args) == 0 {
				continue
			}
			st := ii.TraitRef.Args[0]
			if st.Kind != TAdt || st.Def != d {
				continue
			}
			if ii.Negative {
				return false
			}
			s := ic.snapshot()
			_, ok, _ := ic.matchImpl(impl, &TraitRef{Trait: trait, Args: []Ty{ty}}, depth+1)
			ic.rollback(s)
			return ok
		}
	}
	all := func(k string, args []Ty) bool {
		for _, a := range args {
			if a.Kind == TConst || (a.Kind == TParam && a.Sub == 1) {
				continue
			}
			if !ic.autoHolds(k, a, depth+1, seen) {
				return false
			}
		}
		return true
	}
	if t.isStd(d) {
		mod := ""
		if d.Parent != nil {
			mod = d.Parent.Name
		}
		switch d.Name {
		case "Rc":
			if mod == "rc" {
				return false
			}
		case "Weak":
			if mod == "rc" {
				return false
			}
			return all("Send", ty.Args) && all("Sync", ty.Args) // sync::Weak
		case "NonNull":
			return false
		case "Arc":
			return all("Send", ty.Args) && all("Sync", ty.Args)
		case "Mutex", "ReentrantLock":
			return all("Send", ty.Args)
		case "RwLock", "OnceLock", "LazyLock":
			if kind == "Sync" {
				return all("Send", ty.Args) && all("Sync", ty.Args)
			}
			return all("Send", ty.Args)
		case "MutexGuard", "RwLockReadGuard", "RwLockWriteGuard", "ReentrantLockGuard":
			if kind == "Send" {
				return false
			}
			return all("Sync", ty.Args)
		case "Cell", "RefCell", "UnsafeCell", "OnceCell", "SyncUnsafeCell":
			if kind == "Sync" && d.Name != "SyncUnsafeCell" {
				return false
			}
			return all("Send", ty.Args)
		case "Receiver":
			if kind == "Sync" {
				return false
			}
			return all("Send", ty.Args)
		case "Sender", "SyncSender":
			return all("Send", ty.Args)
		case "AtomicPtr", "JoinHandle", "Thread", "ScopedJoinHandle":
			return true
		case "Ref", "RefMut":
			if mod == "cell" && kind == "Send" {
				return false
			}
		}
		return all(kind, ty.Args)
	}
	// user types: all fields
	t.AdtFields(d)
	if d.Adt == nil {
		return true
	}
	for _, v := range d.Adt.Variants {
		for _, f := range v.Fields {
			ft := Subst(f.Ty, ty.Args)
			if ft.HasError() {
				continue
			}
			if ft.HasProj() {
				ft = ic.tryNormalize(ft)
			}
			if !ic.autoHolds(kind, ft, depth+1, seen) {
				return false
			}
		}
	}
	return true
}

// stdAutoTrait finds std's Send or Sync trait.
func (t *Tcx) stdAutoTrait(kind string) *Def {
	if t.autoTraits == nil {
		t.autoTraits = map[string]*Def{}
		for _, d := range t.R.Defs {
			if d.Kind == DefTrait && t.autoKind(d) != "" {
				t.autoTraits[d.Name] = d
			}
		}
	}
	return t.autoTraits[kind]
}
