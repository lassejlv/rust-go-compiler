package expand

import "strings"

// A port of rustc_ast_pretty's pretty printer (Oppen's algorithm, pp.rs),
// used by stringify! so that long token streams wrap where rustc's do
// (78-column margin, boxes for delimited groups).

const (
	ppMargin       = 78
	ppMinSpace     = 60
	ppSizeInfinity = 0xffff
	ppIndentUnit   = 4
)

type ppKind uint8

const (
	ppString ppKind = iota
	ppBreak
	ppBegin
	ppEnd
)

type ppToken struct {
	kind ppKind
	s    string
	// Break
	blank, offset int
	// Begin
	indent     int
	consistent bool
}

type ppEntry struct {
	tok  ppToken
	size int
}

type ppFrame struct {
	fits       bool
	consistent bool
	indent     int // Broken frames: the indentation to restore
}

type ppPrinter struct {
	out         strings.Builder
	space       int
	buf         []ppEntry // ring buffer: buf[i] has index bufBase+i
	bufBase     int
	leftTotal   int
	rightTotal  int
	scanStack   []int // indices into buf (absolute)
	printStack  []ppFrame
	indent      int
	pendingIndt int
	noWrap      bool // never break lines
}

func newPP() *ppPrinter { return &ppPrinter{space: ppMargin} }

func (p *ppPrinter) entry(i int) *ppEntry { return &p.buf[i-p.bufBase] }

func (p *ppPrinter) push(e ppEntry) int {
	p.buf = append(p.buf, e)
	return p.bufBase + len(p.buf) - 1
}

func (p *ppPrinter) clearBuf() {
	p.bufBase += len(p.buf)
	p.buf = p.buf[:0]
}

func (p *ppPrinter) word(s string) { p.scanString(s) }
func (p *ppPrinter) space1()       { p.scanBreak(1, 0) }
func (p *ppPrinter) breakOffset(n, off int) {
	p.scanBreak(n, off)
}
func (p *ppPrinter) ibox(indent int) { p.scanBegin(ppToken{kind: ppBegin, indent: indent}) }
func (p *ppPrinter) cbox(indent int) {
	p.scanBegin(ppToken{kind: ppBegin, indent: indent, consistent: true})
}
func (p *ppPrinter) end() { p.scanEnd() }

func (p *ppPrinter) eof() string {
	if len(p.scanStack) > 0 {
		p.checkStack(0)
		p.advanceLeft()
	}
	return p.out.String()
}

func (p *ppPrinter) scanBegin(t ppToken) {
	if len(p.scanStack) == 0 {
		p.leftTotal, p.rightTotal = 1, 1
		p.clearBuf()
	}
	p.scanStack = append(p.scanStack, p.push(ppEntry{tok: t, size: -p.rightTotal}))
}

func (p *ppPrinter) scanEnd() {
	if len(p.scanStack) == 0 {
		p.printEnd()
		return
	}
	p.scanStack = append(p.scanStack, p.push(ppEntry{tok: ppToken{kind: ppEnd}, size: -1}))
}

func (p *ppPrinter) scanBreak(blank, offset int) {
	if len(p.scanStack) == 0 {
		p.leftTotal, p.rightTotal = 1, 1
		p.clearBuf()
	} else {
		p.checkStack(0)
	}
	p.scanStack = append(p.scanStack, p.push(ppEntry{tok: ppToken{kind: ppBreak, blank: blank, offset: offset}, size: -p.rightTotal}))
	p.rightTotal += blank
}

func (p *ppPrinter) scanString(s string) {
	if len(p.scanStack) == 0 {
		p.printString(s)
		return
	}
	p.push(ppEntry{tok: ppToken{kind: ppString, s: s}, size: len(s)})
	p.rightTotal += len(s)
	p.checkStream()
}

func (p *ppPrinter) checkStream() {
	for p.rightTotal-p.leftTotal > p.space {
		if len(p.scanStack) > 0 && p.scanStack[0] == p.bufBase {
			p.scanStack = p.scanStack[1:]
			p.buf[0].size = ppSizeInfinity
		}
		p.advanceLeft()
		if len(p.buf) == 0 {
			break
		}
	}
}

func (p *ppPrinter) advanceLeft() {
	for len(p.buf) > 0 && p.buf[0].size >= 0 {
		left := p.buf[0]
		p.buf = p.buf[1:]
		p.bufBase++
		switch left.tok.kind {
		case ppString:
			p.leftTotal += len(left.tok.s)
			p.printString(left.tok.s)
		case ppBreak:
			p.leftTotal += left.tok.blank
			p.printBreak(left.tok, left.size)
		case ppBegin:
			p.printBegin(left.tok, left.size)
		case ppEnd:
			p.printEnd()
		}
	}
}

func (p *ppPrinter) checkStack(depth int) {
	for len(p.scanStack) > 0 {
		idx := p.scanStack[len(p.scanStack)-1]
		e := p.entry(idx)
		switch e.tok.kind {
		case ppBegin:
			if depth == 0 {
				return
			}
			p.scanStack = p.scanStack[:len(p.scanStack)-1]
			e.size += p.rightTotal
			depth--
		case ppEnd:
			p.scanStack = p.scanStack[:len(p.scanStack)-1]
			e.size = 1
			depth++
		default:
			p.scanStack = p.scanStack[:len(p.scanStack)-1]
			e.size += p.rightTotal
			if depth == 0 {
				return
			}
		}
	}
}

func (p *ppPrinter) top() ppFrame {
	if len(p.printStack) == 0 {
		return ppFrame{} // broken, inconsistent, indent 0
	}
	return p.printStack[len(p.printStack)-1]
}

func (p *ppPrinter) printBegin(t ppToken, size int) {
	if size > p.space {
		p.printStack = append(p.printStack, ppFrame{consistent: t.consistent, indent: p.indent})
		p.indent += t.indent
		if p.indent < 0 {
			p.indent = 0
		}
	} else {
		p.printStack = append(p.printStack, ppFrame{fits: true, consistent: t.consistent})
	}
}

func (p *ppPrinter) printEnd() {
	if len(p.printStack) == 0 {
		return
	}
	f := p.printStack[len(p.printStack)-1]
	p.printStack = p.printStack[:len(p.printStack)-1]
	if !f.fits {
		p.indent = f.indent
	}
}

func (p *ppPrinter) printBreak(t ppToken, size int) {
	top := p.top()
	fits := top.fits || (!top.consistent && size <= p.space) || p.noWrap
	if fits {
		p.pendingIndt += t.blank
		p.space -= t.blank
		return
	}
	p.out.WriteByte('\n')
	indent := p.indent + t.offset
	if indent < 0 {
		indent = 0
	}
	p.pendingIndt = indent
	p.space = ppMargin - indent
	if p.space < ppMinSpace {
		p.space = ppMinSpace
	}
}

func (p *ppPrinter) printString(s string) {
	for ; p.pendingIndt > 0; p.pendingIndt-- {
		p.out.WriteByte(' ')
	}
	p.out.WriteString(s)
	p.space -= len(s)
}
