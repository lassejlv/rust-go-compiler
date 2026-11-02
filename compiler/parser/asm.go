package parser

import (
	"rustgo/compiler/ast"
	"rustgo/compiler/lexer"
	"strings"
)

// isAsmMac reports whether a macro call is `asm!` / `core::arch::asm!`.
func isAsmMac(mac *ast.MacCall) bool {
	segs := mac.Path.Segments
	return len(segs) > 0 && segs[len(segs)-1].Ident.Name == "asm"
}

// parseAsm turns the tokens of an asm! invocation into an AsmExpr.
func (p *Parser) parseAsm(mac *ast.MacCall, eb ast.ExprBase) ast.Expr {
	sub := NewFromTokenTrees(mac.Args)
	x := &ast.AsmExpr{ExprBase: eb}
	var tmpl []string
	for !sub.atEOF() {
		t := sub.cur()
		switch {
		case t.Kind == lexer.Str && len(x.Operands) == 0 && x.Options == nil:
			sub.next()
			tmpl = append(tmpl, unquoteStrTok(t))
		case sub.is("options") && sub.isN(1, "("):
			sub.next()
			g := sub.parseDelimGroup()
			for _, tt := range g.Inner {
				if tt.Delim == 0 && tt.Tok.Kind == lexer.Ident {
					x.Options = append(x.Options, tt.Tok.Text)
				}
			}
		case sub.is("clobber_abi") && sub.isN(1, "("):
			sub.next()
			g := sub.parseDelimGroup()
			for _, tt := range g.Inner {
				if tt.Delim == 0 && tt.Tok.Kind == lexer.Str {
					x.ClobberABI = append(x.ClobberABI, unquoteStrTok(tt.Tok))
				}
			}
		default:
			x.Operands = append(x.Operands, sub.parseAsmOperand())
		}
		if !sub.eat(",") {
			break
		}
	}
	if !sub.atEOF() {
		sub.errExpected("`,` in asm!")
	}
	x.Template = strings.Join(tmpl, "\n")
	return x
}

func unquoteStrTok(t lexer.Token) string {
	// Text holds the decoded string contents for Str tokens
	return t.Text
}

func (p *Parser) parseAsmOperand() *ast.AsmOperand {
	lo := p.lo()
	op := &ast.AsmOperand{}
	if p.cur().Kind == lexer.Ident && p.isN(1, "=") && !p.isN(1, "==") {
		op.Name = p.cur().Text
		p.next()
		p.next()
	}
	kind := p.cur().Text
	switch kind {
	case "in", "out", "lateout", "inout", "inlateout":
		p.next()
		g := p.parseDelimGroup()
		if len(g.Inner) == 1 {
			t := g.Inner[0].Tok
			op.Reg = t.Text
			op.Explicit = t.Kind == lexer.Str
		}
		op.Kind = kind
		e := p.parseExpr()
		switch kind {
		case "in":
			op.In = e
		case "out", "lateout":
			if _, discard := e.(*ast.UnderscoreExpr); !discard {
				op.Out = e
			}
		default: // inout(reg) x [=> y]
			op.In = e
			if p.eat("=>") {
				o := p.parseExpr()
				if _, discard := o.(*ast.UnderscoreExpr); !discard {
					op.Out = o
				}
			} else {
				op.Out = e
			}
		}
	case "const":
		p.next()
		op.Kind = "const"
		op.In = p.parseExpr()
	case "sym":
		p.next()
		op.Kind = "sym"
		op.In = p.parseExpr()
	default:
		p.errExpected("asm! operand")
	}
	op.Span = p.span(lo)
	return op
}
