use crate::{
    compiler::Compiler,
    interpreter::representation::{
        ArithmeticOperator, BooleanOperator, ComparisonOperator, IRExpr, IRType, IRValue, LValue,
    },
};
use std::{
    collections::{HashMap, HashSet},
    fmt::{Display, Write},
};

pub struct CCompiler {
    constants: HashMap<LValue, LValue>,
    functypes: HashSet<IRType>,
    typedefs: String,
}

enum CompileResult {
    Source(String),
    LValue(LValue),
}

impl Display for CompileResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(s) => write!(f, "{s}"),
            Self::LValue(l) => write!(f, "{l}"),
        }
    }
}

impl Compiler for CCompiler {
    type Output = String;
    type Error = String;

    fn compile(mut self, expr: &IRValue) -> Result<Self::Output, Self::Error> {
        let mut prelude = String::new();
        let expr = self.compile_expr(expr, &mut prelude)?;
        Ok(format!(
            "#include <stdio.h>\n{}\nint main(){{{prelude}printf(\"%i\\n\",{expr});}}",
            self.typedefs
        ))
    }
}

impl CCompiler {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            functypes: HashSet::new(),
            typedefs: String::new(),
        }
    }

    fn short_type(&mut self, ty: &IRType) -> String {
        match ty {
            IRType::String => "str".to_string(),
            ty => self.write_type(ty),
        }
    }

    fn write_type(&mut self, ty: &IRType) -> String {
        match ty {
            IRType::Int | IRType::Boolean => "int".to_string(),
            IRType::Float => "float".to_string(),
            IRType::String => "char *".to_string(),
            IRType::Function { inputs, output } => {
                let mut short_name = format!("_func_{}", self.short_type(output));
                for i in inputs {
                    short_name.push('_');
                    short_name.push_str(&self.short_type(i));
                }
                short_name.push('_');
                if !self.functypes.contains(ty) {
                    self.functypes.insert(ty.clone());
                    let mut typedef =
                        format!("typedef {}(*{short_name})(", self.write_type(output));
                    let mut first = true;
                    for i in inputs {
                        if first {
                            first = false;
                        } else {
                            typedef.push(',');
                        }
                        let short_ty = self.write_type(i);
                        typedef.push_str(&short_ty);
                    }
                    typedef.push_str(");\n");
                    self.typedefs.push_str(&typedef);
                }
                short_name
            }
        }
    }

    fn compile_expr(
        &mut self,
        expr: &IRValue,
        prelude: &mut String,
    ) -> Result<CompileResult, String> {
        match &expr.0 {
            IRExpr::GetLocal(lvalue) => Ok(CompileResult::LValue(
                self.constants.get(lvalue).copied().unwrap_or(*lvalue),
            )),
            IRExpr::SetLocal(lvalue, value, body) => {
                let ty = value.1.clone();
                let val = self.compile_expr(value, prelude)?;
                match val {
                    CompileResult::LValue(lv) => {
                        self.constants.insert(*lvalue, lv);
                    }
                    CompileResult::Source(val) => {
                        write!(prelude, "{} {lvalue}={val};", self.write_type(&ty)).unwrap();
                    }
                }
                self.compile_expr(body, prelude)
            }
            IRExpr::Int(i) => Ok(CompileResult::Source(format!("{i}"))),
            IRExpr::Float(f) => Ok(CompileResult::Source(format!("{f}"))),
            IRExpr::String(s) => Ok(CompileResult::Source(format!("{s:?}"))),
            IRExpr::If {
                condition,
                body,
                else_body,
            } => {
                let cond = self.compile_expr(condition, prelude)?;
                let mut body_prelude = String::new();
                let body = self.compile_expr(body, &mut body_prelude)?;
                let mut else_prelude = String::new();
                let else_body = self.compile_expr(else_body, &mut else_prelude)?;
                let lvalue = LValue::new();
                write!(prelude, "{} {lvalue};if({cond}){{{body_prelude}{lvalue}={body};}}else{{{else_prelude}{lvalue}={else_body};}}", self.write_type(&expr.1)).unwrap();

                Ok(CompileResult::LValue(lvalue))
            }
            IRExpr::Arithmetic(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude)?;
                let rhs = self.compile_expr(rhs, prelude)?;
                match op {
                    ArithmeticOperator::Add => Ok(CompileResult::Source(format!("({lhs}+{rhs})"))),
                    ArithmeticOperator::Sub => Ok(CompileResult::Source(format!("({lhs}-{rhs})"))),
                    ArithmeticOperator::Div => Ok(CompileResult::Source(format!("({lhs}/{rhs})"))),
                    ArithmeticOperator::Mul => Ok(CompileResult::Source(format!("({lhs}*{rhs})"))),
                    ArithmeticOperator::Mod => Ok(CompileResult::Source(format!("({lhs}%{rhs})"))),
                }
            }
            IRExpr::Comparison(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude)?;
                let rhs = self.compile_expr(rhs, prelude)?;
                match op {
                    ComparisonOperator::Eq => Ok(CompileResult::Source(format!("({lhs}=={rhs})"))),
                    ComparisonOperator::Ne => Ok(CompileResult::Source(format!("({lhs}!={rhs})"))),
                    ComparisonOperator::Le => Ok(CompileResult::Source(format!("({lhs}<={rhs})"))),
                    ComparisonOperator::Ge => Ok(CompileResult::Source(format!("({lhs}>={rhs})"))),
                    ComparisonOperator::Lt => Ok(CompileResult::Source(format!("({lhs}<{rhs})"))),
                    ComparisonOperator::Gt => Ok(CompileResult::Source(format!("({lhs}>{rhs})"))),
                }
            }
            IRExpr::Boolean(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude)?;
                let rhs = self.compile_expr(rhs, prelude)?;
                match op {
                    BooleanOperator::And => Ok(CompileResult::Source(format!("({lhs}&&{rhs})"))),
                    BooleanOperator::Or => Ok(CompileResult::Source(format!("({lhs}||{rhs})"))),
                }
            }
            IRExpr::Function {
                params,
                body,
                captures,
            } => {
                let funcname = LValue::new();
                let IRType::Function { inputs, output } = &expr.1 else {
                    return Err(format!("Expected function type; got `{:?}`", expr.1));
                };
                let mut funcdef = format!("{} {funcname}(", self.write_type(output));
                let mut first = true;
                for (input, param) in inputs.iter().zip(params.iter()) {
                    if first {
                        first = false;
                    } else {
                        funcdef.push(',');
                    }
                    write!(funcdef, "{} {param}", self.write_type(input))
                        .map_err(|e| e.to_string())?;
                }
                write!(funcdef, "){{").map_err(|e| e.to_string())?;
                let body = self.compile_expr(body, &mut funcdef)?;
                writeln!(funcdef, "return {body};}}").map_err(|e| e.to_string())?;
                self.typedefs.push_str(&funcdef);
                Ok(CompileResult::LValue(funcname))
            }
            IRExpr::FunctionCall(func, args) => {
                let mut funcall = format!("{}", self.compile_expr(func, prelude)?);
                funcall.push('(');
                let mut first = true;
                for arg in args {
                    if first {
                        first = false;
                    } else {
                        funcall.push(',');
                    }
                    funcall.push_str(&format!("{}", self.compile_expr(arg, prelude)?));
                }
                funcall.push(')');
                println!("{func:?}{args:?}");
                Ok(CompileResult::Source(funcall))
            }
        }
    }
}
