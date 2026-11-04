package ast

import "sync"

// `pub(in path)` visibilities other than `pub(in crate)`, `pub(in self)`
// and `pub(in super)` carry their path: they are encoded as values from
// visRestrictedBase up, indexing restrictedPaths. VisRestricted itself is a
// restricted visibility whose path is unknown (treated as crate-visible).
const visRestrictedBase Visibility = 1 << 20

var (
	restrictedMu    sync.Mutex
	restrictedPaths []*Path
)

// RestrictedVis returns the visibility `pub(in path)`.
func RestrictedVis(path *Path) Visibility {
	restrictedMu.Lock()
	defer restrictedMu.Unlock()
	restrictedPaths = append(restrictedPaths, path)
	return visRestrictedBase + Visibility(len(restrictedPaths)-1)
}

// Restricted reports whether v is a `pub(in path)` visibility.
func (v Visibility) Restricted() bool {
	return v == VisRestricted || v >= visRestrictedBase
}

// RestrictedPath returns the path of a `pub(in path)` visibility (nil if
// v is not one or its path is unknown).
func (v Visibility) RestrictedPath() *Path {
	if v < visRestrictedBase {
		return nil
	}
	restrictedMu.Lock()
	defer restrictedMu.Unlock()
	i := int(v - visRestrictedBase)
	if i < len(restrictedPaths) {
		return restrictedPaths[i]
	}
	return nil
}
