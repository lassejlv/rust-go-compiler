package borrowck

import (
	"fmt"
	"os"
)

// Loans returned to the caller (NLL problem case #3): a loan whose
// reference flows into the return value outlives a universal region of
// the signature, which contains every point of the body. Under NLL such a
// loan is live at every point reachable from where it is created (until
// the borrowed place's base is reassigned), whether or not the path
// returns it:
//
//	match map.get_mut(&k) {
//	    Some(v) => v,
//	    None => { map.insert(k, v); ... } // E0499: still borrowed
//	}
//
// Only reborrows through the function's reference parameters are
// considered (loans of locals returned are E0515 anyway), and only when
// the return value's slots are labeled with lifetimes by the signature.

// computeReturnLive computes lc.retLive: per block, the returned loans
// live at its entry.
func (lc *loanChecker) computeReturnLive(in []*lstate) {
	body := lc.body
	if lc.regions == nil || !lc.sigSlots || body.ClosureKind >= 0 {
		return
	}
	ret := newBitset(lc.nLoans)
	any := false
	// the loans of the values stored into the return place (the
	// parameters' StorageDead at the end of the body kills their loans
	// before the return itself)
	lc.onRetStore = func(out *lstate) {
		for j, lb := range lc.retLabels {
			if lb == "" || lb[0] != '\'' {
				continue
			}
			h := out.holds[skey(0, j)]
			if h == nil {
				continue
			}
			for li := 0; li < lc.nLoans; li++ {
				if !h.has(li) {
					continue
				}
				ln := lc.loans[li]
				if ln.origin != "" || ln.reserve || ln.place.Local < 0 || body.Locals[ln.place.Local].Kind != LArg || !placeThroughRef(ln.place) {
					continue
				}
				ret.set(li)
				any = true
			}
		}
	}
	for bi := range body.Blocks {
		if in[bi] != nil {
			lc.runBlock(bi, in[bi], false)
		}
	}
	lc.onRetStore = nil
	if !any {
		return
	}
	if debugLoans {
		var ls []string
		for li := 0; li < lc.nLoans; li++ {
			if ret.has(li) {
				ls = append(ls, fmt.Sprintf("%d:%s@%v", li, body.placeString(lc.loans[li].place), lc.loans[li].span.Lo))
			}
		}
		fmt.Fprintf(os.Stderr, "borrowck: returned loans %v\n", ls)
	}
	lc.retLoans = ret
	n := len(body.Blocks)
	lc.retLive = make([]bitset, n)
	lc.retLive[0] = newBitset(lc.nLoans)
	work := []int{0}
	queued := make([]bool, n)
	queued[0] = true
	for len(work) > 0 {
		bi := work[0]
		work = work[1:]
		queued[bi] = false
		cur := lc.retLive[bi].clone()
		bl := body.Blocks[bi]
		for si, s := range bl.Stmts {
			lc.retLiveStmt(cur, bi, si, s)
		}
		lc.retLiveTerm(cur, bl.Term)
		for _, sc := range bl.Term.Targets {
			if lc.retLive[sc] == nil {
				lc.retLive[sc] = cur.clone()
			} else if !lc.retLive[sc].union(cur) {
				continue
			}
			if !queued[sc] {
				queued[sc] = true
				work = append(work, sc)
			}
		}
	}
}

// retLiveStmt applies statement s to the returned loans live (kills like
// transferStmt's, then the loan s creates).
func (lc *loanChecker) retLiveStmt(cur bitset, bi, si int, s Stmt) {
	switch s.Kind {
	case SAssign:
		if len(s.Place.Proj) == 0 {
			for _, li := range lc.derefLoans[s.Place.Local] {
				cur.clear(li)
			}
		}
		if li, ok := lc.loanAt[[2]int{bi, si}]; ok && lc.retLoans.has(li) {
			cur.set(li)
		}
	case SStorageDead:
		for _, li := range lc.localLoans[s.Place.Local] {
			cur.clear(li)
		}
	}
}

func (lc *loanChecker) retLiveTerm(cur bitset, t Term) {
	if t.Kind == TCall && len(t.Dest.Proj) == 0 {
		for _, li := range lc.derefLoans[t.Dest.Local] {
			cur.clear(li)
		}
	}
}
