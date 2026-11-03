package borrowck

import "strings"

// paramIndex returns the position of local l among the parameters (-1:
// not a parameter).
func (lc *loanChecker) paramIndex(l LocalID) int {
	for i, p := range lc.params {
		if p == l {
			return i
		}
	}
	return -1
}

// paramCopy returns the parameter a temporary is a copy of (assigned once,
// from the whole parameter), or l itself.
func (lc *loanChecker) paramCopy(l LocalID) LocalID {
	if lc.body.Locals[l].Kind != LTemp {
		return l
	}
	if lc.tempSrc == nil {
		lc.tempSrc = map[LocalID]LocalID{}
		count := map[LocalID]int{}
		for _, bl := range lc.body.Blocks {
			for _, s := range bl.Stmts {
				if s.Kind != SAssign || len(s.Place.Proj) != 0 {
					continue
				}
				count[s.Place.Local]++
				if s.Rv.Kind == RUse && len(s.Rv.Ops) == 1 && s.Rv.Ops[0].Kind != OConst && len(s.Rv.Ops[0].Place.Proj) == 0 {
					lc.tempSrc[s.Place.Local] = s.Rv.Ops[0].Place.Local
				}
			}
			if t := bl.Term; t.Kind == TCall && len(t.Dest.Proj) == 0 {
				count[t.Dest.Local]++
			}
		}
		for t := range lc.tempSrc {
			if count[t] != 1 {
				delete(lc.tempSrc, t)
			}
		}
	}
	if p, ok := lc.tempSrc[l]; ok && lc.body.Locals[p].Kind == LArg {
		return p
	}
	return l
}

// checkAssignStore checks the assignment of a parameter into a place
// behind a reference of another parameter (`*v = s`, `p.x = s`,
// `*self.slot = s`, or `v[0] = s` through the result of IndexMut, a
// reference whose one loan borrows a parameter's referent) against the
// lifetimes the destination parameter's type declares there. Only whole,
// immutable parameters are checked as sources: the regions they hold are
// exactly those of their declared type, so imprecise flows cannot make
// the check reject valid stores.
func (lc *loanChecker) checkAssignStore(st *lstate, s Stmt) {
	if s.Rv.Kind != RUse || len(s.Rv.Ops) != 1 {
		return
	}
	o := s.Rv.Ops[0]
	if o.Kind == OConst || len(o.Place.Proj) != 0 {
		return
	}
	src := lc.paramCopy(o.Place.Local)
	si := lc.paramIndex(src)
	if si < 0 || si >= len(lc.paramLabels) || lc.body.Locals[src].Mut {
		return
	}
	srcLay := lc.localLayout(src)
	if ot := lc.body.Locals[o.Place.Local].Ty; ot == nil || layoutOf(ot).key != srcLay.key {
		return
	}
	srcLabels := lc.paramLabels[si]
	if srcLay.flat || srcLay.n() == 0 || len(srcLabels) != srcLay.n() || srcLay.key == "" {
		return
	}
	// the first deref of a reference in the destination
	di := -1
	for i, pr := range s.Place.Proj {
		if pr.Kind != PDeref {
			continue
		}
		if pr.Deref != DRef && pr.Deref != DRefMut {
			return
		}
		di = i
		break
	}
	if di < 0 {
		return
	}
	dest := s.Place
	if lc.paramIndex(s.Place.Local) < 0 {
		if lc.body.Locals[s.Place.Local].Kind != LTemp {
			return
		}
		base := Place{Local: s.Place.Local, Proj: s.Place.Proj[:di]}
		held := lc.depthView(lc.placeVal(st, base))[0]
		found := false
		for li := 0; li < lc.nLoans; li++ {
			if !held.has(li) || lc.loans[li].origin != "" {
				continue
			}
			if found {
				return // more than one place
			}
			found = true
			p := lc.loans[li].place
			if p.Local < 0 {
				return
			}
			proj := append(append([]Proj(nil), p.Proj...), s.Place.Proj[di+1:]...)
			dest = Place{Local: p.Local, Proj: proj}
		}
		if !found {
			return
		}
	}
	pi := lc.paramIndex(dest.Local)
	if pi < 0 || pi >= len(lc.paramLabels) {
		return
	}
	refs := 0
	for _, pr := range dest.Proj {
		if pr.Kind == PDeref {
			if pr.Deref != DRef && pr.Deref != DRefMut && pr.Deref != DBox {
				return
			}
			if pr.Deref != DBox {
				refs++
			}
		}
	}
	if refs == 0 {
		return
	}
	info := lc.placeInfo(dest)
	if info.raw || info.lay.flat || len(info.m) != info.lay.n() {
		return
	}
	var starts []int
	if info.lay.key == srcLay.key && info.lay.n() == srcLay.n() {
		starts = []int{0}
	} else {
		for _, n := range findKey(info.lay.root, srcLay.key) {
			if n.end-n.start == srcLay.n() {
				starts = append(starts, n.start)
			}
		}
	}
	if len(starts) == 0 {
		return
	}
	labels := lc.paramLabels[pi]
	for j, origin := range srcLabels {
		if origin == "" || origin[0] != '\'' || origin == "'static" || origin == "'?" || strings.HasPrefix(origin, "'#h") {
			continue
		}
		set := map[string]bool{}
		any := false
		for _, st := range starts {
			x := st + j
			if x >= len(info.m) || len(info.m[x]) == 0 {
				any = true
				continue
			}
			for _, r := range info.m[x] {
				lb := ""
				if r < len(labels) {
					lb = labels[r]
				}
				switch {
				case lb == "" || lb[0] != '\'' || strings.HasPrefix(lb, "'#h"):
					any = true
				case lb == "'?":
					if lc.regions.elided == "" {
						any = true
					} else {
						set[lc.regions.elided] = true
					}
				default:
					set[lb] = true
				}
			}
		}
		if any || len(set) == 0 {
			continue
		}
		if !lc.regions.originOK(origin, set) {
			lc.regionError(origin, s.Span)
			return
		}
	}
}
