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
    constants: HashMap<LValue, CompileResult>,
    typedecls: HashSet<IRType>,
    typedefs: String,
}

#[derive(Clone)]
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
            "#include <stdio.h>\n#include <stdlib.h>\n{}\nint main(){{{prelude}printf(\"%i\\n\",{expr});}}",
            self.typedefs
        ))
    }
}

impl CCompiler {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            typedecls: HashSet::new(),
            typedefs: String::new(),
        }
    }

    fn short_type(&mut self, ty: &IRType) -> String {
        match ty {
            IRType::String => "str".to_string(),
            ty => self.write_type(ty).replace("*", ""),
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
                if !self.typedecls.contains(ty) {
                    self.typedecls.insert(ty.clone());
                    let mut lambda_typedecl = format!(
                        "typedef struct {short_name} {{{} (*f)(struct {short_name}*",
                        self.write_type(output)
                    );
                    for input in inputs {
                        lambda_typedecl.push(',');
                        lambda_typedecl.push_str(&self.write_type(input));
                    }
                    writeln!(
                        self.typedefs,
                        "{lambda_typedecl}); void (*d)(struct {short_name}*); int refcount;}} {short_name};"
                    )
                    .unwrap();
                }
                format!("{}*", short_name)
            }
        }
    }

    fn compile_expr(
        &mut self,
        expr: &IRValue,
        prelude: &mut String,
    ) -> Result<CompileResult, String> {
        match &expr.0 {
            IRExpr::GetLocal(lvalue) => Ok(self
                .constants
                .get(lvalue)
                .cloned()
                .unwrap_or(CompileResult::LValue(*lvalue))),
            IRExpr::SetLocal(lvalue, value, body) => {
                let ty = value.1.clone();
                let val = self.compile_expr(value, prelude)?;
                match val {
                    CompileResult::LValue(_) => {
                        self.constants.insert(*lvalue, val);
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
                let IRType::Function { inputs, output } = &expr.1 else {
                    return Err(format!("Expected function type; got `{:?}`", expr.1));
                };
                let output_ty = self.write_type(output);
                let funcname = LValue::new();
                let lambda_ty = self.write_type(&expr.1);

                let captures_ty = if captures.is_empty() {
                    None
                } else {
                    let captures_ty = format!("_captures_{funcname}");

                    // create the typedef for the captures type
                    let mut captures_typedecl = format!(
                        "typedef struct {{{lambda_ty} lambda;",
                        lambda_ty = lambda_ty.replace("*", "")
                    );
                    for (cap, typ) in captures {
                        let typ = self.write_type(typ);
                        write!(captures_typedecl, "{typ} {cap};").unwrap();
                    }
                    writeln!(self.typedefs, "{captures_typedecl}}} {captures_ty};").unwrap();
                    Some(captures_ty)
                };

                // define the function
                let mut funcdef = format!(
                    "{output_ty} {funcname}({lambda_ty} {captures_name}",
                    captures_name = if captures_ty.is_some() {
                        "captures_tmp"
                    } else {
                        "captures"
                    }
                );
                for (input, param) in inputs.iter().zip(params.iter()) {
                    write!(funcdef, ",{} {param}", self.write_type(input)).unwrap();
                }
                for (lv, _) in captures {
                    self.constants
                        .insert(*lv, CompileResult::Source(format!("(captures->{lv})")));
                }
                funcdef.push_str("){");
                if let Some(captures_ty) = &captures_ty {
                    write!(
                        funcdef,
                        "{captures_ty} *captures = ({captures_ty}*)captures_tmp;"
                    )
                    .unwrap();
                }
                // compile the function body
                let body = self.compile_expr(body, &mut funcdef)?;
                writeln!(self.typedefs, "{funcdef}return {body};}}").unwrap();
                let lambda_lv = LValue::new();
                // allocate the lambda and captures
                match captures_ty {
                    Some(captures_ty) => {
                        writeln!(
                            prelude,
                            "{captures_ty} *{lambda_lv} = malloc(sizeof (*{lambda_lv}));\n{lambda_lv}->lambda.f = {funcname};\n{lambda_lv}->lambda.refcount = 1;"
                        )
                        .unwrap();
                        for (cap, _) in captures {
                            writeln!(prelude, "{lambda_lv}->{cap}={cap};").unwrap();
                        }
                        Ok(CompileResult::Source(format!(
                            "(({lambda_ty}){lambda_lv})"
                        )))
                    }
                    None => {
                        writeln!(
                            prelude,
                            "{lambda_ty} {lambda_lv} = malloc(sizeof (*{lambda_lv}));\n{lambda_lv}->f = {funcname};\n{lambda_lv}->refcount = 1;"
                        )
                        .unwrap();
                        Ok(CompileResult::LValue(lambda_lv))
                    }
                }
            }
            IRExpr::FunctionCall(func, args) => {
                let func_compiled = match self.compile_expr(func, prelude)? {
                    CompileResult::LValue(l) => CompileResult::LValue(l),
                    CompileResult::Source(s) => {
                        let lv = LValue::new();
                        write!(prelude, "{} {lv} = {s};", self.write_type(&func.1)).unwrap();
                        CompileResult::LValue(lv)
                    }
                };
                let args_compiled = args
                    .iter()
                    .map(|v| self.compile_expr(v, prelude))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut funcall = format!("{}->f({},", func_compiled, func_compiled);
                let mut first = true;
                for arg in &args_compiled {
                    if first {
                        first = false;
                    } else {
                        funcall.push(',');
                    }
                    funcall.push_str(&format!("{}", arg));
                }
                funcall.push(')');
                // println!("{func:?}{args:?}");
                Ok(CompileResult::Source(funcall))
            }
        }
    }
}
