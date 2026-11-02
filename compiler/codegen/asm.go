package codegen

import (
	"fmt"
	"strconv"
	"strings"

	"rustgo/compiler/ast"
	. "rustgo/compiler/middle"
	"rustgo/compiler/typeck"
)

// asm! → GCC extended asm. Templates keep Rust's Intel syntax (the program
// is compiled with -masm=intel); operands get symbolic names %[oN].

// explicit x86-64 registers that have a GCC constraint letter
var asmRegLetter = map[string]string{
	"rax": "a", "eax": "a", "ax": "a", "al": "a",
	"rbx": "b", "ebx": "b", "bx": "b", "bl": "b",
	"rcx": "c", "ecx": "c", "cx": "c", "cl": "c",
	"rdx": "d", "edx": "d", "dx": "d", "dl": "d",
	"rsi": "S", "esi": "S", "si": "S", "sil": "S",
	"rdi": "D", "edi": "D", "di": "D", "dil": "D",
}

var asmClass = map[string]string{
	"reg": "r", "reg_abcd": "Q", "reg_byte": "q",
	"xmm_reg": "x", "ymm_reg": "x", "zmm_reg": "v", "kreg": "k",
}

// canonical 64-bit name of an explicit register (for clobbers)
func asmCanonReg(r string) string {
	switch r {
	case "eax", "ax", "al", "ah":
		return "rax"
	case "ebx", "bx", "bl", "bh":
		return "rbx"
	case "ecx", "cx", "cl", "ch":
		return "rcx"
	case "edx", "dx", "dl", "dh":
		return "rdx"
	case "esi", "si", "sil":
		return "rsi"
	case "edi", "di", "dil":
		return "rdi"
	}
	if strings.HasPrefix(r, "r") && strings.HasSuffix(r, "d") && len(r) <= 4 {
		return strings.TrimSuffix(r, "d")
	}
	return r
}

func (fc *fnGen) asm(x *ast.AsmExpr) {
	fc.g.usesAsm = true
	type gop struct {
		name, constraint, expr string
		isOut                  bool
	}
	var outs, ins []gop
	var post []string // assignments after the asm
	used := map[string]bool{}
	names := make([]string, len(x.Operands))
	byName := map[string]int{}
	for i, op := range x.Operands {
		name := fmt.Sprintf("o%d", i)
		names[i] = name
		if op.Name != "" {
			byName[op.Name] = i
		}
		cons := asmClass[op.Reg]
		var regVar string // register variable for explicit regs without a letter
		if op.Explicit {
			if l, ok := asmRegLetter[op.Reg]; ok {
				cons = l
			} else {
				cons = "r"
				regVar = op.Reg
			}
			used[asmCanonReg(op.Reg)] = true
		}
		if cons == "" {
			cons = "r"
		}
		decl := func(t Ty, init string) string {
			v := fc.newName("asmv")
			ct := fc.ctype(t)
			reg := ""
			if regVar != "" {
				reg = fmt.Sprintf(" __asm__(%q)", regVar)
			}
			if init != "" {
				fc.line("register %s %s%s = %s;", ct, v, reg, init)
			} else {
				fc.line("register %s %s%s;", ct, v, reg)
			}
			return v
		}
		switch op.Kind {
		case "in":
			t := fc.aty(op.In)
			v := decl(t, fc.value(op.In))
			ins = append(ins, gop{name, cons, v, false})
		case "const":
			ins = append(ins, gop{name, "i", fc.value(op.In), false})
		case "sym":
			sym := ""
			if pe, ok := op.In.(*ast.PathExpr); ok {
				if vr := fc.r.Paths[pe.ID]; vr != nil && vr.Kind == typeck.VFn {
					sym = fc.g.fnName(vr.Def, vr.Args)
				} else if vr != nil && vr.Kind == typeck.VStatic {
					sym = "&" + fc.g.staticVar(vr.Def)
				}
			}
			if sym == "" {
				fc.g.fatalf("%s: unsupported `sym` operand in asm!", op.Span.Lo)
			}
			ins = append(ins, gop{name, "i", sym, false})
		case "out", "lateout":
			if op.Out == nil {
				if op.Explicit {
					// `out("rcx") _`: just a clobber
					names[i] = ""
					continue
				}
				// discarded class register: needs some output anyway
				v := decl(TyU64, "")
				_ = v
				outs = append(outs, gop{name, "=" + cons, v, true})
				continue
			}
			t := fc.aty(op.Out)
			v := decl(t, "")
			c := "=" + cons
			if op.Kind == "out" && !op.Explicit {
				c = "=&" + cons
			}
			outs = append(outs, gop{name, c, v, true})
			post = append(post, fmt.Sprintf("%s = %s;", fc.asPlace(fc.adjusted(op.Out)).s, v))
		case "inout", "inlateout":
			t := fc.aty(op.In)
			v := decl(t, fc.value(op.In))
			c := "+" + cons
			if op.Kind == "inout" && op.Out != nil && op.Out != op.In && !op.Explicit {
				c = "+&" + cons
			}
			outs = append(outs, gop{name, c, v, true})
			if op.Out != nil {
				post = append(post, fmt.Sprintf("%s = %s;", fc.asPlace(fc.adjusted(op.Out)).s, v))
			}
		}
	}
	attSyntax := false
	for _, o := range x.Options {
		if o == "att_syntax" {
			attSyntax = true
		}
	}
	// template: {N}, {name}, {} and :modifiers → %[oN]; {{ }} → { }; % → %%
	var b strings.Builder
	tmpl := x.Template
	next := 0
	for i := 0; i < len(tmpl); i++ {
		c := tmpl[i]
		switch {
		case c == '{' && i+1 < len(tmpl) && tmpl[i+1] == '{':
			b.WriteByte('{')
			i++
		case c == '}' && i+1 < len(tmpl) && tmpl[i+1] == '}':
			b.WriteByte('}')
			i++
		case c == '{':
			j := strings.IndexByte(tmpl[i:], '}')
			if j < 0 {
				fc.g.fatalf("%s: invalid asm! template", x.Span.Lo)
			}
			spec := tmpl[i+1 : i+j]
			i += j
			ref, mod := spec, ""
			if k := strings.IndexByte(spec, ':'); k >= 0 {
				ref, mod = spec[:k], spec[k+1:]
			}
			idx := -1
			switch {
			case ref == "":
				idx = next
				next++
			default:
				if n, err := strconv.Atoi(ref); err == nil {
					idx = n
				} else if n, ok := byName[ref]; ok {
					idx = n
				}
			}
			if idx < 0 || idx >= len(names) || names[idx] == "" {
				fc.g.fatalf("%s: asm! template refers to unknown operand `{%s}`", x.Span.Lo, spec)
			}
			gm := ""
			switch mod {
			case "e":
				gm = "k"
			case "x":
				gm = "w"
			case "l":
				gm = "b"
			case "h":
				gm = "h"
			case "r":
				gm = "q"
			}
			if x.Operands[idx].Kind == "const" || x.Operands[idx].Kind == "sym" {
				gm = "c"
			} else if attSyntax {
				// operands print Intel-style (`rax`); AT&T needs `%rax`
				b.WriteString("%%")
			}
			fmt.Fprintf(&b, "%%%s[%s]", gm, names[idx])
		case c == '%':
			b.WriteString("%%")
		default:
			b.WriteByte(c)
		}
	}
	text := b.String()
	opts := map[string]bool{}
	for _, o := range x.Options {
		opts[o] = true
	}
	if opts["att_syntax"] {
		text = ".att_syntax prefix\n\t" + text + "\n\t.intel_syntax noprefix"
	}
	// clobbers
	var clob []string
	for _, op := range x.Operands {
		if op.Explicit && op.Out == nil && (op.Kind == "out" || op.Kind == "lateout") {
			clob = append(clob, fmt.Sprintf("%q", asmCanonReg(op.Reg)))
		}
	}
	for _, abi := range x.ClobberABI {
		_ = abi // C / sysv64 / system: caller-saved registers
		for _, r := range []string{"rax", "rcx", "rdx", "rsi", "rdi", "r8", "r9", "r10", "r11"} {
			if !used[r] {
				clob = append(clob, fmt.Sprintf("%q", r))
			}
		}
		for k := 0; k < 16; k++ {
			clob = append(clob, fmt.Sprintf("\"xmm%d\"", k))
		}
	}
	if !opts["nomem"] && !opts["readonly"] {
		clob = append(clob, `"memory"`)
	}
	if !opts["preserves_flags"] {
		clob = append(clob, `"cc"`)
	}
	render := func(ops []gop) string {
		var parts []string
		for _, o := range ops {
			parts = append(parts, fmt.Sprintf("[%s] %q (%s)", o.name, o.constraint, o.expr))
		}
		return strings.Join(parts, ", ")
	}
	vol := "__volatile__ "
	if opts["pure"] && len(outs) > 0 {
		vol = ""
	}
	fc.line("__asm__ %s(%s : %s : %s : %s);", vol, cQuote(text), render(outs), render(ins), strings.Join(clob, ", "))
	for _, p := range post {
		fc.line("%s", p)
	}
	if opts["noreturn"] {
		fc.line("__builtin_unreachable();")
	}
}

// cQuote renders s as a C string literal.
func cQuote(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range []byte(s) {
		switch r {
		case '"':
			b.WriteString(`\"`)
		case '\\':
			b.WriteString(`\\`)
		case '\n':
			b.WriteString(`\n`)
		case '\t':
			b.WriteString(`\t`)
		default:
			b.WriteByte(r)
		}
	}
	b.WriteByte('"')
	return b.String()
}
