package borrowck

import (
	"fmt"
	"os"
	"runtime/debug"

	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// Ctx carries per-check state.
type Ctx struct {
	report bool
	tcx    *typeck.Tcx
	fn     *Def // the function being checked (its signature's regions)
}

// CheckFn borrow-checks a function instance (and the closures in it) and
// returns the diagnostics, already formatted.
func CheckFn(tcx *typeck.Tcx, d *Def, r *typeck.Results) (msgs []string) {
	if d.Fn == nil || d.Fn.Ast == nil || d.Fn.Ast.Body == nil || r == nil {
		return nil
	}
	defer func() {
		if e := recover(); e != nil {
			if os.Getenv("RUSTGO_BORROWCK_DEBUG") != "" {
				fmt.Fprintf(os.Stderr, "borrowck: internal error in %s: %v\n%s", d.Path(), e, debug.Stack())
			}
			msgs = nil
		}
	}()
	b := buildFn(tcx, d, r)
	bodies := append([]*Body{b.body}, b.closures...)
	dump := os.Getenv("RUSTGO_MIR_DUMP")
	for _, body := range bodies {
		if dump != "" && (dump == "1" || dump == d.Name) {
			fmt.Fprint(os.Stderr, body.String())
		}
		c := &checker{body: body, md: buildMoveData(body), seen: map[string]bool{}, ctx: &Ctx{tcx: tcx, fn: d}}
		c.checkMoves()
		c.checkLoans()
		msgs = append(msgs, body.Errors...)
		for _, dg := range c.diags {
			msgs = append(msgs, dg.msg)
		}
	}
	return msgs
}
