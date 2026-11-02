package ast

// BaseExpr returns the embedded ExprBase (lets generic code set Span/Attrs).
func (e *ExprBase) BaseExpr() *ExprBase { return e }

// BasePat returns the embedded PatBase.
func (p *PatBase) BasePat() *PatBase { return p }

// BaseType returns the embedded TypeBase.
func (t *TypeBase) BaseType() *TypeBase { return t }
