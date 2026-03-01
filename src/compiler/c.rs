use crate::{
    compiler::Compiler,
    interpreter::representation::{
        ArithmeticOperator, BooleanOperator, ComparisonOperator, IRExpr, IRType, IRValue, LValue,
    },
};
use std::fmt::Write;

pub struct CCompiler;

impl Compiler for CCompiler {
    type Output = String;

    fn compile(self, expr: &IRValue) -> Self::Output {
        let mut prelude = String::new();
        let expr = self.compile_expr(expr, &mut prelude);
        format!("#include <stdio.h>\nint main(){{{prelude}printf(\"%i\\n\",{expr});}}")
    }
}

impl CCompiler {
    fn write_type(&self, ty: &IRType) -> String {
        match ty {
            IRType::Int | IRType::Boolean => "int".to_string(),
            IRType::Float => "float".to_string(),
            IRType::String => "char *".to_string(),
        }
    }

    fn compile_expr(&self, expr: &IRValue, prelude: &mut String) -> String {
        match &expr.0 {
            IRExpr::GetLocal(lvalue) => format!("{lvalue}"),
            IRExpr::SetLocal(lvalue, value, body) => {
                let ty = value.1.clone();
                let val = self.compile_expr(value, prelude);
                write!(prelude, "{} {lvalue}={val};", self.write_type(&ty)).unwrap();
                self.compile_expr(body, prelude)
            }
            IRExpr::Int(i) => format!("{i}"),
            IRExpr::Float(f) => format!("{f}"),
            IRExpr::String(s) => format!("{s:?}"),
            IRExpr::If {
                condition,
                body,
                else_body,
            } => {
                let cond = self.compile_expr(condition, prelude);
                let mut body_prelude = String::new();
                let body = self.compile_expr(body, &mut body_prelude);
                let mut else_prelude = String::new();
                let else_body = self.compile_expr(else_body, &mut else_prelude);
                let lvalue = LValue::new();
                write!(prelude, "{} {lvalue};if({cond}){{{body_prelude}{lvalue}={body};}}else{{{else_prelude}{lvalue}={else_body};}}", self.write_type(&expr.1)).unwrap();

                format!("{lvalue}")
            }
            IRExpr::Arithmetic(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude);
                let rhs = self.compile_expr(rhs, prelude);
                match op {
                    ArithmeticOperator::Add => format!("({lhs}+{rhs})"),
                    ArithmeticOperator::Sub => format!("({lhs}-{rhs})"),
                    ArithmeticOperator::Div => format!("({lhs}/{rhs})"),
                    ArithmeticOperator::Mul => format!("({lhs}*{rhs})"),
                    ArithmeticOperator::Mod => format!("({lhs}%{rhs})"),
                }
            }
            IRExpr::Comparison(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude);
                let rhs = self.compile_expr(rhs, prelude);
                match op {
                    ComparisonOperator::Eq => format!("({lhs}=={rhs})"),
                    ComparisonOperator::Ne => format!("({lhs}!={rhs})"),
                    ComparisonOperator::Le => format!("({lhs}<={rhs})"),
                    ComparisonOperator::Ge => format!("({lhs}>={rhs})"),
                    ComparisonOperator::Lt => format!("({lhs}<{rhs})"),
                    ComparisonOperator::Gt => format!("({lhs}>{rhs})"),
                }
            }
            IRExpr::Boolean(lhs, op, rhs) => {
                let lhs = self.compile_expr(lhs, prelude);
                let rhs = self.compile_expr(rhs, prelude);
                match op {
                    BooleanOperator::And => format!("({lhs}&&{rhs})"),
                    BooleanOperator::Or => format!("({lhs}||{rhs})"),
                }
            }
        }
    }
}
