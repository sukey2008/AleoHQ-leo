// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use super::*;
use crate::parse_ast;
use leo_errors::{CompilerError, ParserError, ParserWarning, Result};
use leo_span::source_map::FileName;
use leo_span::symbol::with_session_globals;

use std::fs;

impl ParserContext<'_> {
    /// Returns a [`Program`] AST if all tokens can be consumed and represent a valid Leo program.
    pub fn parse_program(&mut self) -> Result<Program> {
        let mut imports = IndexMap::new();
        let mut program_scopes = IndexMap::new();

        // TODO: Condense logic
        while self.has_next() {
            match &self.token.token {
                Token::Import => {
                    let (id, import) = self.parse_import()?;
                    imports.insert(id, import);
                }
                Token::Program => {
                    let program_scope = self.parse_program_scope()?;
                    program_scopes.insert((program_scope.name, program_scope.network), program_scope);
                }
                _ => return Err(Self::unexpected_item(&self.token).into()),
            }
        }
        Ok(Program {
            imports,
            program_scopes,
        })
    }

    fn unexpected_item(token: &SpannedToken) -> ParserError {
        ParserError::unexpected(
            &token.token,
            [Token::Function, Token::Struct, Token::Identifier(sym::test)]
                .iter()
                .map(|x| format!("'{}'", x))
                .collect::<Vec<_>>()
                .join(", "),
            token.span,
        )
    }

    /// Parses an import statement `import foo.leo;`.
    pub(super) fn parse_import(&mut self) -> Result<(Identifier, Program)> {
        // Parse `import`.
        let _start = self.expect(&Token::Import)?;

        // Parse `foo`.
        let import_name = self.expect_identifier()?;

        // Parse `.leo`.
        self.expect(&Token::Dot)?;
        if !self.eat(&Token::Leo) {
            // Throw error for non-leo files.
            return Err(ParserError::leo_imports_only(self.token.span).into());
        }

        let _end = self.expect(&Token::Semicolon)?;

        // Tokenize and parse import file.
        // Todo: move this to a different module.
        let mut import_file_path =
            std::env::current_dir().map_err(|err| CompilerError::cannot_open_cwd(err, self.token.span))?;
        import_file_path.push("imports");
        import_file_path.push(format!("{}.leo", import_name.name));

        // Throw an error if the import file doesn't exist.
        if !import_file_path.exists() {
            return Err(CompilerError::import_not_found(import_file_path.display(), self.prev_token.span).into());
        }

        // Read the import file into string.
        // Todo: protect against cyclic imports.
        let program_string =
            fs::read_to_string(&import_file_path).map_err(|e| CompilerError::file_read_error(&import_file_path, e))?;

        // Create import file name.
        let name: FileName = FileName::Real(import_file_path);

        // Register the source (`program_string`) in the source map.
        let prg_sf = with_session_globals(|s| s.source_map.new_source(&program_string, name));

        // Use the parser to construct the imported abstract syntax tree (ast).
        let program_ast = parse_ast(self.handler, &prg_sf.src, prg_sf.start_pos)?;

        Ok((import_name, program_ast.into_repr()))
    }

    /// Parsers a program scope `program foo.aleo { ... }`.
    fn parse_program_scope(&mut self) -> Result<ProgramScope> {
        // Parse `program` keyword.
        let _start = self.expect(&Token::Program)?;

        // Parse the program name.
        let name = self.expect_identifier()?;

        // Parse the program network.
        self.expect(&Token::Dot)?;

        let network = self.expect_identifier()?;

        // Parse `{`.
        let _open = self.expect(&Token::OpenBrace)?;

        // Parse the body of the program scope.
        let mut functions = IndexMap::new();
        let mut structs = IndexMap::new();
        let mut mappings = IndexMap::new();

        while self.has_next() {
            match &self.token.token {
                Token::Struct | Token::Record => {
                    let (id, struct_) = self.parse_struct()?;
                    structs.insert(id, struct_);
                }
                Token::Mapping => {
                    let (id, mapping) = self.parse_mapping()?;
                    mappings.insert(id, mapping);
                }
                Token::At | Token::Function | Token::Transition => {
                    let (id, function) = self.parse_function()?;
                    functions.insert(id, function);
                }
                Token::Circuit => return Err(ParserError::circuit_is_deprecated(self.token.span).into()),
                Token::CloseBrace => break,
                _ => return Err(Self::unexpected_item(&self.token).into()),
            }
        }

        // Parse `}`.
        let _close = self.expect(&Token::CloseBrace)?;

        Ok(ProgramScope {
            name,
            network,
            functions,
            structs,
            mappings,
        })
    }

    /// Returns a [`Vec<Member>`] AST node if the next tokens represent a struct member.
    fn parse_struct_members(&mut self) -> Result<(Vec<Member>, Span)> {
        let mut members = Vec::new();

        let (mut semi_colons, mut commas) = (false, false);

        while !self.check(&Token::RightCurly) {
            let variable = self.parse_member_variable_declaration()?;

            if self.eat(&Token::Semicolon) {
                if commas {
                    self.emit_err(ParserError::mixed_commas_and_semicolons(self.token.span));
                }
                semi_colons = true;
            }

            if self.eat(&Token::Comma) {
                if semi_colons {
                    self.emit_err(ParserError::mixed_commas_and_semicolons(self.token.span));
                }
                commas = true;
            }

            members.push(variable);
        }
        let span = self.expect(&Token::RightCurly)?;

        Ok((members, span))
    }

    /// Parses `IDENT: TYPE`.
    pub(super) fn parse_typed_ident(&mut self) -> Result<(Identifier, Type)> {
        let name = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let type_ = self.parse_type()?.0;

        Ok((name, type_))
    }

    /// Returns a [`Member`] AST node if the next tokens represent a struct member variable.
    fn parse_member_variable_declaration(&mut self) -> Result<Member> {
        let (identifier, type_) = self.parse_typed_ident()?;

        Ok(Member { identifier, type_ })
    }

    /// Parses a struct or record definition, e.g., `struct Foo { ... }` or `record Foo { ... }`.
    pub(super) fn parse_struct(&mut self) -> Result<(Identifier, Struct)> {
        let is_record = matches!(&self.token.token, Token::Record);
        let start = self.expect_any(&[Token::Struct, Token::Record])?;
        let struct_name = self.expect_identifier()?;

        self.expect(&Token::LeftCurly)?;
        let (members, end) = self.parse_struct_members()?;

        Ok((
            struct_name,
            Struct {
                identifier: struct_name,
                members,
                is_record,
                span: start + end,
            },
        ))
    }

    /// Parses a mapping declaration, e.g. `mapping balances: address => u128`.
    pub(super) fn parse_mapping(&mut self) -> Result<(Identifier, Mapping)> {
        let start = self.expect(&Token::Mapping)?;
        let identifier = self.expect_identifier()?;
        self.expect(&Token::Colon)?;
        let (key_type, _) = self.parse_type()?;
        self.expect(&Token::BigArrow)?;
        let (value_type, _) = self.parse_type()?;
        let end = self.expect(&Token::Semicolon)?;
        Ok((
            identifier,
            Mapping {
                identifier,
                key_type,
                value_type,
                span: start + end,
            },
        ))
    }

    /// Returns a [`ParamMode`] AST node if the next tokens represent a function parameter mode.
    pub(super) fn parse_mode(&mut self) -> Result<Mode> {
        // TODO: Allow explicit "private" mode.
        let public = self.eat(&Token::Public).then_some(self.prev_token.span);
        let constant = self.eat(&Token::Constant).then_some(self.prev_token.span);
        let const_ = self.eat(&Token::Const).then_some(self.prev_token.span);

        if let Some(span) = const_ {
            self.emit_warning(ParserWarning::const_parameter_or_input(span));
        }

        match (public, constant, const_) {
            (None, Some(_), None) => Ok(Mode::Const),
            (None, None, Some(_)) => Ok(Mode::Const),
            (None, None, None) => Ok(Mode::None),
            (Some(_), None, None) => Ok(Mode::Public),
            (Some(m1), Some(m2), None) | (Some(m1), None, Some(m2)) | (None, Some(m1), Some(m2)) => {
                Err(ParserError::inputs_multiple_variable_types_specified(m1 + m2).into())
            }
            (Some(m1), Some(m2), Some(m3)) => {
                Err(ParserError::inputs_multiple_variable_types_specified(m1 + m2 + m3).into())
            }
        }
    }

    /// Returns a [`Input`] AST node if the next tokens represent a function output.
    fn parse_input(&mut self) -> Result<functions::Input> {
        let mode = self.parse_mode()?;
        let name = self.expect_identifier()?;
        self.expect(&Token::Colon)?;

        if self.peek_is_external() {
            let external = self.expect_identifier()?;
            let mut span = name.span + external.span;

            // Parse `.leo/`.
            self.eat(&Token::Dot);
            self.eat(&Token::Leo);
            self.eat(&Token::Div);

            // Parse record name.
            let record = self.expect_identifier()?;

            // Parse `.record`.
            self.eat(&Token::Dot);
            self.eat(&Token::Record);
            span = span + self.prev_token.span;

            Ok(functions::Input::External(External {
                identifier: name,
                program_name: external,
                record,
                span,
            }))
        } else {
            let type_ = self.parse_type()?.0;

            Ok(functions::Input::Internal(FunctionInput {
                identifier: name,
                mode,
                type_,
                span: name.span,
            }))
        }
    }

    /// Returns a [`FunctionOutput`] AST node if the next tokens represent a function output.
    fn parse_function_output(&mut self) -> Result<FunctionOutput> {
        // TODO: Could this span be made more accurate?
        let mode = self.parse_mode()?;
        let (type_, span) = self.parse_type()?;
        Ok(FunctionOutput { mode, type_, span })
    }

    /// Returns a [`Output`] AST node if the next tokens represent a function output.
    fn parse_output(&mut self) -> Result<Output> {
        if self.peek_is_external() {
            let external = self.expect_identifier()?;
            let mut span = external.span;

            // Parse `.leo/`.
            self.eat(&Token::Dot);
            self.eat(&Token::Leo);
            self.eat(&Token::Div);

            // Parse record name.
            let record = self.expect_identifier()?;

            // Parse `.record`.
            self.eat(&Token::Dot);
            self.eat(&Token::Record);
            span = span + self.prev_token.span;

            Ok(Output::External(External {
                identifier: Identifier::new(Symbol::intern("dummy")),
                program_name: external,
                record,
                span,
            }))
        } else {
            Ok(Output::Internal(self.parse_function_output()?))
        }
    }

    fn peek_is_external(&self) -> bool {
        matches!(
            (&self.token.token, self.look_ahead(1, |t| &t.token)),
            (Token::Identifier(_), Token::Dot)
        )
    }

    /// Returns an [`Annotation`] AST node if the next tokens represent an annotation.
    fn parse_annotation(&mut self) -> Result<Annotation> {
        // Parse the `@` symbol and identifier.
        let start = self.expect(&Token::At)?;
        let identifier = self.expect_identifier()?;
        let span = start + identifier.span;

        // TODO: Verify that this check is sound.
        // Check that there is no whitespace in between the `@` symbol and identifier.
        match identifier.span.hi.0 - start.lo.0 > 1 + identifier.name.to_string().len() as u32 {
            true => Err(ParserError::space_in_annotation(span).into()),
            false => Ok(Annotation { identifier, span }),
        }
    }

    /// Returns an [`(Identifier, Function)`] AST node if the next tokens represent a function name
    /// and function definition.
    fn parse_function(&mut self) -> Result<(Identifier, Function)> {
        // TODO: Handle dangling annotations.
        // TODO: Handle duplicate annotations.
        // Parse annotations, if they exist.
        let mut annotations = Vec::new();
        while self.look_ahead(0, |t| &t.token) == &Token::At {
            annotations.push(self.parse_annotation()?)
        }
        // Parse `<call_type> IDENT`, where `<call_type>` is `function` or `transition`.
        let (call_type, start) = match self.token.token {
            Token::Function => (CallType::Standard, self.expect(&Token::Function)?),
            Token::Transition => (CallType::Transition, self.expect(&Token::Transition)?),
            _ => self.unexpected("'function', 'transition'")?,
        };
        let name = self.expect_identifier()?;

        // Parse parameters.
        let (inputs, ..) = self.parse_paren_comma_list(|p| p.parse_input().map(Some))?;

        // Parse return type.
        let output = match self.eat(&Token::Arrow) {
            false => vec![],
            true => {
                self.disallow_struct_construction = true;
                let output = match self.peek_is_left_par() {
                    true => self.parse_paren_comma_list(|p| p.parse_output().map(Some))?.0,
                    false => vec![self.parse_output()?],
                };
                self.disallow_struct_construction = false;
                output
            }
        };

        // Parse the function body.
        let block = self.parse_block()?;

        // Parse the `finalize` block if it exists.
        let finalize = match self.eat(&Token::Finalize) {
            false => None,
            true => {
                // Get starting span.
                let start = self.prev_token.span;

                // Parse the identifier.
                let identifier = self.expect_identifier()?;

                // Parse parameters.
                let (input, ..) = self.parse_paren_comma_list(|p| p.parse_input().map(Some))?;

                // Parse return type.
                let output = match self.eat(&Token::Arrow) {
                    false => vec![],
                    true => {
                        self.disallow_struct_construction = true;
                        let output = match self.peek_is_left_par() {
                            true => self.parse_paren_comma_list(|p| p.parse_output().map(Some))?.0,
                            false => vec![self.parse_output()?],
                        };
                        self.disallow_struct_construction = false;
                        output
                    }
                };

                // Parse the finalize body.
                let block = self.parse_block()?;
                let span = start + block.span;

                Some(Finalize::new(identifier, input, output, block, span))
            }
        };

        let span = start + block.span;
        Ok((
            name,
            Function::new(annotations, call_type, name, inputs, output, block, finalize, span),
        ))
    }
}

use leo_span::{sym, Symbol};
