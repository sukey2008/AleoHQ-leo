// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the Leo library.

// The Leo library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Leo library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the Leo library. If not, see <https://www.gnu.org/licenses/>.

//! Compiles a Leo program from a file path.

use crate::CompilerOptions;
use leo_asg::{
    ArrayAccessExpression as AsgArrayAccessExpression,
    ArrayInitExpression as AsgArrayInitExpression,
    ArrayInlineExpression as AsgArrayInlineExpression,
    ArrayRangeAccessExpression as AsgArrayRangeAccessExpression,
    BinaryExpression as AsgBinaryExpression,
    CallExpression as AsgCallExpression,
    CastExpression as AsgCastExpression,
    CircuitAccessExpression as AsgCircuitAccessExpression,
    CircuitInitExpression as AsgCircuitInitExpression,
    Constant as AsgConstant,
    Expression as AsgExpression,
    TernaryExpression as AsgTernaryExpression,
    TupleAccessExpression as AsgTupleAccessExpression,
    TupleInitExpression as AsgTupleInitExpression,
    UnaryExpression as AsgUnaryExpression,
    VariableRef as AsgVariableRef,
};
use leo_ast::{
    ArrayAccessExpression as AstArrayAccessExpression,
    ArrayInitExpression as AstArrayInitExpression,
    ArrayInlineExpression as AstArrayInlineExpression,
    ArrayRangeAccessExpression as AstArrayRangeAccessExpression,
    BinaryExpression as AstBinaryExpression,
    CallExpression as AstCallExpression,
    CanonicalizeError,
    CastExpression as AstCastExpression,
    CircuitInitExpression as AstCircuitInitExpression,
    CircuitMemberAccessExpression as AstCircuitAccessExpression,
    Expression as AstExpression,
    ReconstructingReducer,
    SpreadOrExpression,
    TernaryExpression as AstTernaryExpression,
    TupleAccessExpression as AstTupleAccessExpression,
    TupleInitExpression as AstTupleInitExpression,
    UnaryExpression as AstUnaryExpression,
    ValueExpression,
};

pub struct CombineAstAsgDirector<R: ReconstructingReducer> {
    ast_reducer: R,
    options: CompilerOptions,
    in_circuit: bool,
}

impl<R: ReconstructingReducer> CombineAstAsgDirector<R> {
    pub fn new(ast_reducer: R, options: CompilerOptions) -> Self {
        Self {
            ast_reducer,
            options,
            in_circuit: false,
        }
    }

    pub fn reduce_expression(
        &mut self,
        ast: &AstExpression,
        asg: &AsgExpression,
    ) -> Result<AstExpression, CanonicalizeError> {
        let new = match (ast, asg) {
            // AsgExpression::Identifier(identifier) => AsgExpression::Identifier(self.reduce_identifier(&identifier)?),
            // AsgExpression::Value(value) => AsgExpression::Value(self.reduce_value(&value)?),
            // AsgExpression::Binary(binary) => AsgExpression::Binary(self.reduce_binary(&binary)?),
            // AsgExpression::Unary(unary) => AsgExpression::Unary(self.reduce_unary(&unary)?),
            // AsgExpression::Ternary(ternary) => AsgExpression::Ternary(self.reduce_ternary(&ternary)?),
            // AsgExpression::Cast(cast) => AsgExpression::Cast(self.reduce_cast(&cast)?),

            // AsgExpression::ArrayInline(array_inline) => AsgExpression::ArrayInline(self.reduce_array_inline(&array_inline)?),
            // AsgExpression::ArrayInit(array_init) => AsgExpression::ArrayInit(self.reduce_array_init(&array_init)?),
            // AsgExpression::ArrayAccess(array_access) => AsgExpression::ArrayAccess(self.reduce_array_access(&array_access)?),
            // AsgExpression::ArrayRangeAccess(array_range_access) => {
            //     AsgExpression::ArrayRangeAccess(self.reduce_array_range_access(&array_range_access)?)
            // }

            // AsgExpression::TupleInit(tuple_init) => AsgExpression::TupleInit(self.reduce_tuple_init(&tuple_init)?),
            // AsgExpression::TupleAccess(tuple_access) => AsgExpression::TupleAccess(self.reduce_tuple_access(&tuple_access)?),

            // AsgExpression::CircuitInit(circuit_init) => AsgExpression::CircuitInit(self.reduce_circuit_init(&circuit_init)?),
            // AsgExpression::CircuitMemberAccess(circuit_member_access) => {
            //     AsgExpression::CircuitMemberAccess(self.reduce_circuit_member_access(&circuit_member_access)?)
            // }
            // AsgExpression::CircuitStaticFunctionAccess(circuit_static_fn_access) => {
            //     AsgExpression::CircuitStaticFunctionAccess(
            //         self.reduce_circuit_static_fn_access(&circuit_static_fn_access)?,
            //     )
            // }

            // AsgExpression::Call(call) => AsgExpression::Call(self.reduce_call(&call)?),
            _ => ast.clone(),
        };

        self.ast_reducer.reduce_expression(ast, new, self.in_circuit)
    }

    pub fn reduce_array_access(
        &mut self,
        ast: &AstArrayAccessExpression,
        asg: &AsgArrayAccessExpression,
    ) -> Result<AstArrayAccessExpression, CanonicalizeError> {
        let array = self.reduce_expression(&ast.array, asg.array.get())?;
        let index = self.reduce_expression(&ast.index, asg.index.get())?;

        self.ast_reducer.reduce_array_access(ast, array, index, self.in_circuit)
    }

    pub fn reduce_array_init(
        &mut self,
        ast: &AstArrayInitExpression,
        asg: &AsgArrayInitExpression,
    ) -> Result<AstArrayInitExpression, CanonicalizeError> {
        let element = self.reduce_expression(&ast.element, asg.element.get())?;

        self.ast_reducer.reduce_array_init(ast, element, self.in_circuit)
    }

    pub fn reduce_array_inline(
        &mut self,
        ast: &AstArrayInlineExpression,
        asg: &AsgArrayInlineExpression,
    ) -> Result<AstArrayInlineExpression, CanonicalizeError> {
        let mut elements = vec![];
        for (ast_element, asg_element) in ast.elements.iter().zip(asg.elements.iter()) {
            let reduced_element = match ast_element {
                SpreadOrExpression::Expression(ast_expression) => {
                    SpreadOrExpression::Expression(self.reduce_expression(ast_expression, asg_element.0.get())?)
                }
                SpreadOrExpression::Spread(ast_expression) => {
                    SpreadOrExpression::Spread(self.reduce_expression(ast_expression, asg_element.0.get())?)
                }
            };

            elements.push(reduced_element);
        }

        self.ast_reducer.reduce_array_inline(ast, elements, self.in_circuit)
    }

    pub fn reduce_array_range_access(
        &mut self,
        ast: &AstArrayRangeAccessExpression,
        asg: &AsgArrayRangeAccessExpression,
    ) -> Result<AstArrayRangeAccessExpression, CanonicalizeError> {
        let array = self.reduce_expression(&ast.array, asg.array.get())?;
        let left = match (ast.left.as_ref(), asg.left.get()) {
            (Some(ast_left), Some(asg_left)) => Some(self.reduce_expression(ast_left, asg_left)?),
            _ => None,
        };
        let right = match (ast.right.as_ref(), asg.right.get()) {
            (Some(ast_right), Some(asg_right)) => Some(self.reduce_expression(ast_right, asg_right)?),
            _ => None,
        };

        self.ast_reducer
            .reduce_array_range_access(ast, array, left, right, self.in_circuit)
    }

    pub fn reduce_binary(
        &mut self,
        ast: &AstBinaryExpression,
        asg: &AsgBinaryExpression,
    ) -> Result<AstBinaryExpression, CanonicalizeError> {
        let left = self.reduce_expression(&ast.left, asg.left.get())?;
        let right = self.reduce_expression(&ast.right, asg.right.get())?;

        self.ast_reducer
            .reduce_binary(ast, left, right, ast.op.clone(), self.in_circuit)
    }

    pub fn reduce_call(
        &mut self,
        ast: &AstCallExpression,
        asg: &AsgCallExpression,
    ) -> Result<AstCallExpression, CanonicalizeError> {
        // TODO FIGURE IT OUT
        // let function = self.reduce_expression(&ast.function, asg.function.get())?;
        // let target = asg.target.get().map(|exp| self.reduce_expression())

        let mut arguments = vec![];
        for (ast_arg, asg_arg) in ast.arguments.iter().zip(asg.arguments.iter()) {
            arguments.push(self.reduce_expression(ast_arg, asg_arg.get())?);
        }

        Ok(ast.clone())
        // self.ast_reducer.reduce_call(ast, function, arguments, self.in_circuit)
    }

    pub fn reduce_cast_expression(
        &mut self,
        ast: &AstCastExpression,
        asg: &AsgCastExpression,
    ) -> Result<AstCastExpression, CanonicalizeError> {
        let inner = self.reduce_expression(&ast.inner, &asg.inner.get())?;
        // TODO REDUCE TYPE let target_type;

        Ok(ast.clone())

        // self.ast_reducer.reduce_cast(ast, inner, target_type, self.in_circuit)
    }

    pub fn reduce_constant(
        &mut self,
        ast: &ValueExpression,
        asg: &AsgConstant,
    ) -> Result<ValueExpression, CanonicalizeError> {
        // TODO REDUCE GV
        let new = match ast {
            // AstConstant::Group(group_value) => {
            //     AstConstant::Group(Box::new(self.reduce_group_value(&group_value)?))
            // }
            _ => ast.clone(),
        };

        self.ast_reducer.reduce_value(ast, new)
    }

    pub fn reduce_circuit_access(
        &mut self,
        ast: &AstCircuitAccessExpression,
        asg: &AsgCircuitAccessExpression,
    ) -> Result<AstCircuitAccessExpression, CanonicalizeError> {
        // TODO FIGURE IT OUT
        // let circuit = self.reduce_expression(&circuit_member_access.circuit)?;
        // let name = self.reduce_identifier(&circuit_member_access.name)?;
        // let target = input.target.get().map(|e| self.reduce_expression(e));

        Ok(ast.clone())
        // self.reducer
        //     .reduce_circuit_member_access(ast, circuit, name, self.in_circuit)
    }

    pub fn reduce_circuit_init(
        &mut self,
        ast: &AstCircuitInitExpression,
        asg: &AsgCircuitInitExpression,
    ) -> Result<AstCircuitInitExpression, CanonicalizeError> {
        // TODO FIGURE IT OUT
        // let name = self.reduce_identifier(&circuit_init.name)?;
        // let values = asg
        //     .values
        //     .iter()
        //     .map(|(ident, e)| (ident.clone(), self.reduce_expression(e.get())))
        //     .collect();

        // let mut members = vec![];
        // for member in ast.members.iter() {
        //     // members.push(self.reduce_circuit_implied_variable_definition(member)?);
        // }

        Ok(ast.clone())
        // self.ast_reducer
        //     .reduce_circuit_init(ast, name, members, self.in_circuit)
    }

    pub fn reduce_ternary_expression(
        &mut self,
        ast: &AstTernaryExpression,
        asg: &AsgTernaryExpression,
    ) -> Result<AstTernaryExpression, CanonicalizeError> {
        let condition = self.reduce_expression(&ast.condition, asg.condition.get())?;
        let if_true = self.reduce_expression(&ast.if_true, asg.if_true.get())?;
        let if_false = self.reduce_expression(&ast.if_false, asg.if_false.get())?;

        self.ast_reducer
            .reduce_ternary(ast, condition, if_true, if_false, self.in_circuit)
    }

    pub fn reduce_tuple_access(
        &mut self,
        ast: &AstTupleAccessExpression,
        asg: &AsgTupleAccessExpression,
    ) -> Result<AstTupleAccessExpression, CanonicalizeError> {
        let tuple = self.reduce_expression(&ast.tuple, asg.tuple_ref.get())?;

        self.ast_reducer.reduce_tuple_access(ast, tuple, self.in_circuit)
    }

    pub fn reduce_tuple_init(
        &mut self,
        ast: &AstTupleInitExpression,
        asg: &AsgTupleInitExpression,
    ) -> Result<AstTupleInitExpression, CanonicalizeError> {
        let mut elements = vec![];
        for (ast_element, asg_element) in ast.elements.iter().zip(asg.elements.iter()) {
            let element = self.reduce_expression(ast_element, asg_element.get())?;
            elements.push(element);
        }

        self.ast_reducer.reduce_tuple_init(ast, elements, self.in_circuit)
    }

    pub fn reduce_unary(
        &mut self,
        ast: &AstUnaryExpression,
        asg: &AsgUnaryExpression,
    ) -> Result<AstUnaryExpression, CanonicalizeError> {
        let inner = self.reduce_expression(&ast.inner, asg.inner.get())?;

        self.ast_reducer
            .reduce_unary(ast, inner, ast.op.clone(), self.in_circuit)
    }

    pub fn reduce_variable_ref(
        &mut self,
        ast: &ValueExpression,
        asg: &AsgVariableRef,
    ) -> Result<ValueExpression, CanonicalizeError> {
        // TODO FIGURE IT OUT
        let new = match ast {
            // ValueExpression::Group(group_value) => {
            //     ValueExpression::Group(Box::new(self.reduce_group_value(&group_value)?))
            // }
            _ => ast.clone(),
        };

        Ok(new)
        // self.ast_reducer.reduce_value(value, new)
    }

    // pub fn reduce_program(&mut self, ast: &leo_ast::Program, asg: &leo_asg::Program) -> Result<leo_ast::Program, leo_ast::CanonicalizeError> {
    //     pub fn new(ast_reducer: AstR) -> Self {
    //         Self {
    //             ast_reducer
    //         }
    //     }

    //     let mut circuits = IndexMap::new();
    //     for ((asg_ident, asg_circuit), (ast_ident, ast_circuit)) in asg.circuits.iter().zip(&ast.circuits) {
    //         circuits.insert(
    //             self.reduce_identifier(asg_ident, ast_ident),
    //             self.reduce_circuit(asg_circuit, ast_circuit)
    //         );
    //     }

    //     let mut functions = IndexMap::new();
    //     for ((asg_ident, asg_function), (ast_identifier, ast_function)) in asg.functions.iter().zip(&ast.functions) {
    //        // etc
    //     }

    //     self.ast_reducer.reduce_program(ast, ast.expected_input, ast.imports, IndexMap::new(), IndexMap::new())
    // }
}
