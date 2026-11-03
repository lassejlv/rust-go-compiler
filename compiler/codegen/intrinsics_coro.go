package codegen

import (
	_ "embed"
	"fmt"
)

//go:embed coro.h
var coroH string

// coroIntrinsic lowers coroutine intrinsics used by async blocks.
func coroIntrinsic(name string, v []string) (string, bool) {
	switch name {
	case "coro_new":
		return fmt.Sprintf("rg_coro_new((void (*)(uint8_t*))(%s), %s)", v[0], v[1]), true
	case "coro_resume":
		return fmt.Sprintf("rg_coro_resume(%s)", v[0]), true
	case "coro_panicked":
		return fmt.Sprintf("rg_coro_panicked(%s)", v[0]), true
	case "coro_yield":
		return "(rg_coro_yield(), (Unit){})", true
	case "coro_free":
		return fmt.Sprintf("(rg_coro_free(%s), (Unit){})", v[0]), true
	case "coro_get_cx":
		return "rg_coro_cx", true
	case "coro_set_cx":
		return fmt.Sprintf("(rg_coro_cx = (%s), (Unit){})", v[0]), true
	}
	return "", false
}
