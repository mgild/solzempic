//! Procedural macros for the Solzempic framework.
//!
//! This crate provides compile-time code generation for Solana programs built
//! with Solzempic. The macros reduce boilerplate while maintaining zero runtime
//! overhead through compile-time expansion.
//!
//! # Available Macros
//!
//! | Macro | Type | Purpose |
//! |-------|------|---------|
//! | [`SolzempicDispatch`] | Attribute | Dispatch enum + framework types |
//! | [`instruction`] | Attribute | Instruction trait implementations |
//! | [`Account`] | Derive | Account struct with discriminator |
//!
//! # Quick Start
//!
//! ```ignore
//! use solzempic::SolzempicDispatch;
//!
//! // 1. Define dispatch enum (generates framework types)
//! #[SolzempicDispatch("Your11111111111111111111111111111111111111")]
//! pub enum MyInstruction {
//!     Initialize = 0,
//!     Transfer = 1,
//! }
//!
//! // 2. Define instruction struct
//! pub struct Transfer<'a> {
//!     from: AccountRefMut<'a, TokenAccount>,
//!     to: AccountRefMut<'a, TokenAccount>,
//! }
//!
//! // 3. Implement with #[instruction] macro
//! #[instruction(TransferParams)]
//! impl<'a> Transfer<'a> {
//!     fn build(accounts: &'a [AccountInfo], params: &TransferParams) -> Result<Self, ProgramError> {
//!         // Parse accounts...
//!     }
//!
//!     fn validate(&self, program_id: &Pubkey, params: &TransferParams) -> ProgramResult {
//!         // Validate state...
//!     }
//!
//!     fn execute(&self, program_id: &Pubkey, params: &TransferParams) -> ProgramResult {
//!         // Execute logic...
//!     }
//! }
//!
//! // 4. In entrypoint:
//! MyInstruction::process(program_id, accounts, instruction_data)?;
//! ```
//!
//! # Generated Code
//!
//! ## From `SolzempicDispatch`
//!
//! - `ID` - Program ID constant
//! - `Solzempic` - Framework type implementing `Framework` trait
//! - `AccountRef<'a, T>` - Type alias for read-only accounts
//! - `AccountRefMut<'a, T>` - Type alias for writable accounts
//! - `ShardRefContext<'a, T>` - Type alias for read-only shard triplets
//! - `ShardRefMutContext<'a, T>` - Type alias for writable shard triplets
//! - `id()` - Returns `&'static Pubkey`
//! - `TryFrom<u8>` - Discriminator parsing
//! - `dispatch()` - Handler dispatch (after enum construction)
//! - `process()` - Direct dispatch (more efficient)
//!
//! ## From `instruction`
//!
//! - `InstructionParams` impl with associated `Params` type
//! - `Instruction<'a>` impl with `build`, `validate`, `execute` methods
//!
//! ## From `Account` derive
//!
//! - `#[repr(C)]` for stable memory layout
//! - `Clone`, `Copy`, `Pod`, `Zeroable` derives
//! - Prepended `discriminator: [u8; 8]` field
//! - `Loadable` impl for zero-copy loading
//!
//! # Performance
//!
//! All macros expand at compile time with zero runtime cost. The `process()`
//! method is more efficient than `dispatch()` because it avoids constructing
//! the enum variant before dispatching.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Expr, Lit, ItemImpl, ImplItem, ItemStruct, Type};

/// Convert a PascalCase identifier to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Attribute macro for complete Solana program setup.
///
/// This is the main entry point for defining a Solzempic program. It generates
/// everything needed: program ID, framework types, dispatch enum, entrypoint,
/// and process_instruction function.
///
/// # Generated Items
///
/// | Item | Type | Description |
/// |------|------|-------------|
/// | `ID` | `Pubkey` | Program ID constant |
/// | `Solzempic` | `struct` | Framework type implementing `Framework` trait |
/// | `AccountRef<'a, T>` | `type` | Read-only account wrapper alias |
/// | `AccountRefMut<'a, T>` | `type` | Writable account wrapper alias |
/// | `ShardRefContext<'a, T>` | `type` | Read-only shard triplet context alias |
/// | `ShardRefMutContext<'a, T>` | `type` | Writable shard triplet context alias |
/// | `id()` | `fn` | Returns `&'static Pubkey` |
/// | `process_instruction` | `fn` | Program entrypoint handler |
/// | `entrypoint!` | macro | Registers the entrypoint (unless `no-entrypoint` feature) |
///
/// # Example
///
/// ```ignore
/// use solzempic::SolzempicEntrypoint;
///
/// #[SolzempicEntrypoint("Your11111111111111111111111111111111111111")]
/// pub enum MyInstruction {
///     Initialize = 0,
///     Transfer = 1,
/// }
/// ```
///
/// This single attribute generates a complete program setup.
///
/// # Panics
///
/// Compile-time panics if:
/// - Applied to a non-enum type
/// - No program ID provided in attribute
/// - Variant lacks explicit discriminant value
/// Parse account specs from #[accounts(...)] attribute on enum variant.
/// Format: #[accounts(name: constraint, name2: constraint2, ...)]
/// Constraints: mut (writable), signer, mut_signer (both), program, or empty (readonly)
fn parse_variant_accounts(attrs: &[syn::Attribute]) -> Vec<(String, bool, bool, bool)> {
    let mut accounts = Vec::new();

    for attr in attrs {
        if attr.path().is_ident("accounts") {
            // Parse the content as comma-separated name: constraint pairs
            let content = attr.meta.require_list()
                .expect("#[accounts(...)] requires a list");

            let tokens_str = content.tokens.to_string();

            // Parse "name: constraint, name2: constraint2" format
            for part in tokens_str.split(',') {
                let part = part.trim();
                if part.is_empty() { continue; }

                let (name, constraint) = if let Some(colon_pos) = part.find(':') {
                    let name = part[..colon_pos].trim().to_string();
                    let constraint = part[colon_pos + 1..].trim().to_string();
                    (name, constraint)
                } else {
                    (part.to_string(), String::new())
                };

                let is_signer = constraint == "signer" || constraint == "mut_signer";
                let is_writable = constraint == "mut" || constraint == "mut_signer";
                let is_program = constraint == "program";

                accounts.push((name, is_signer, is_writable, is_program));
            }
        }
    }

    accounts
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn SolzempicEntrypoint(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let enum_name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;

    // Parse the program ID from attribute - either a string literal or an identifier
    let program_id_tokens: proc_macro2::TokenStream = if attr.is_empty() {
        panic!("SolzempicEntrypoint requires a program ID, e.g. #[SolzempicEntrypoint(\"Your111...\")]");
    } else {
        let attr_str = attr.to_string();
        let trimmed = attr_str.trim();
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            // String literal: convert to pinocchio_pubkey::pubkey!() call
            let pubkey_str = &trimmed[1..trimmed.len()-1];
            let pubkey_str_lit = syn::LitStr::new(pubkey_str, proc_macro2::Span::call_site());
            quote! { ::pinocchio_pubkey::pubkey!(#pubkey_str_lit) }
        } else {
            // Identifier: use directly
            let ident: syn::Ident = syn::parse(attr.clone())
                .expect("SolzempicEntrypoint attribute must be a string literal or identifier");
            quote! { #ident }
        }
    };

    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => panic!("SolzempicEntrypoint can only be applied to enums"),
    };

    // Collect variant info (name, discriminator, and accounts)
    let variant_info: Vec<_> = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let discriminant = variant.discriminant.as_ref()
            .expect("SolzempicEntrypoint requires explicit discriminant values");
        let disc_expr = &discriminant.1;
        let accounts = parse_variant_accounts(&variant.attrs);
        (variant_name, disc_expr, accounts)
    }).collect();

    // Filter out any ShankInstruction derive from input attrs to avoid conflicts
    let filtered_attrs: Vec<_> = attrs.iter().filter(|attr| {
        if attr.path().is_ident("derive") {
            // Check if the derive contains ShankInstruction
            let content = attr.meta.require_list().ok();
            if let Some(content) = content {
                let tokens_str = content.tokens.to_string();
                !tokens_str.contains("ShankInstruction")
            } else {
                true
            }
        } else {
            true
        }
    }).collect();

    // Generate TryFrom<u8> match arms
    let try_from_arms = variant_info.iter().map(|(name, disc, _)| {
        quote! { #disc => Ok(#enum_name::#name), }
    });

    // Generate dispatch match arms (for backward compat)
    let dispatch_arms = variant_info.iter().map(|(name, _, _)| {
        quote! {
            #enum_name::#name => <#name<'_> as ::solzempic::Instruction<'_>>::process(program_id, accounts, data),
        }
    });

    // Generate process match arms (direct discriminator to handler)
    let process_arms = variant_info.iter().map(|(name, disc, _)| {
        quote! {
            #disc => <#name<'_> as ::solzempic::Instruction<'_>>::process(program_id, accounts, &data[1..]),
        }
    });

    // Generate IDL metadata entries
    let idl_entries = variant_info.iter().map(|(name, disc, _)| {
        quote! {
            ::solzempic::InstructionMeta {
                name: #name::IDL_NAME,
                discriminator: #disc,
                accounts: &#name::SHANK_ACCOUNTS,
                params: #name::IDL_PARAMS,
            }
        }
    });

    // Generate variant definitions for the enum
    let variant_defs = variant_info.iter().map(|(name, disc, _accounts)| {
        quote! {
            #name = #disc
        }
    });

    // Generate enum definition (no ShankInstruction - we generate IDL metadata ourselves)
    let enum_definition = quote! {
        #(#filtered_attrs)*
        #[repr(u8)]
        #vis enum #enum_name {
            #(#variant_defs),*
        }
    };

    // Generate Shank-compatible IDL instruction metadata for each variant
    // This replaces what ShankInstruction derive would generate
    let shank_instruction_metas = variant_info.iter().map(|(name, disc, accounts)| {
        let name_str = name.to_string();
        // Convert PascalCase to snake_case for module name
        let mod_name_str = to_snake_case(&name_str);
        let mod_name = syn::Ident::new(&mod_name_str, name.span());
        let num_accounts = accounts.len();

        // Generate account metadata array
        let account_metas: Vec<proc_macro2::TokenStream> = accounts.iter().enumerate().map(|(idx, (acc_name, is_signer, is_writable, _is_program))| {
            quote! {
                ::solzempic::ShankAccountMeta {
                    index: #idx,
                    name: #acc_name,
                    is_signer: #is_signer,
                    is_writable: #is_writable,
                    is_program: false,
                }
            }
        }).collect();

        quote! {
            /// IDL metadata for #name instruction
            pub mod #mod_name {
                pub const DISCRIMINATOR: u8 = #disc as u8;
                pub const NAME: &str = #name_str;
                pub const ACCOUNTS: [::solzempic::ShankAccountMeta; #num_accounts] = [
                    #(#account_metas),*
                ];
            }
        }
    });

    let expanded = quote! {
        /// Program ID
        pub const ID: ::solana_address::Address = ::solana_address::Address::new_from_array(#program_id_tokens);

        /// Program-specific framework implementation.
        pub struct Solzempic;

        impl ::solzempic::Framework for Solzempic {
            const PROGRAM_ID: ::solana_address::Address = ID;
        }

        /// Read-only account wrapper with ownership validation.
        pub type AccountRef<'a, T> = ::solzempic::AccountRef<'a, T, Solzempic>;

        /// Writable account wrapper with ownership validation.
        pub type AccountRefMut<'a, T> = ::solzempic::AccountRefMut<'a, T, Solzempic>;

        /// Read-only context for sharded data structures.
        pub type ShardRefContext<'a, T> = ::solzempic::ShardRefContext<'a, T, Solzempic>;

        /// Writable context for sharded data structures.
        pub type ShardRefMutContext<'a, T> = ::solzempic::ShardRefMutContext<'a, T, Solzempic>;

        /// Returns the program ID.
        #[inline]
        pub fn id() -> &'static ::solana_address::Address {
            &ID
        }

        #enum_definition

        impl ::core::convert::TryFrom<u8> for #enum_name {
            type Error = ::pinocchio::error::ProgramError;

            #[inline]
            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    #(#try_from_arms)*
                    _ => Err(::pinocchio::error::ProgramError::InvalidInstructionData),
                }
            }
        }

        impl #enum_name {
            /// Dispatch to handler (use after TryFrom conversion)
            #[inline]
            pub fn dispatch(
                self,
                program_id: &::solana_address::Address,
                accounts: &[::pinocchio::AccountView],
                data: &[u8],
            ) -> ::pinocchio::ProgramResult {
                match self {
                    #(#dispatch_arms)*
                }
            }

            /// Process instruction data directly (more efficient - skips enum construction)
            #[inline]
            pub fn process(
                program_id: &::solana_address::Address,
                accounts: &[::pinocchio::AccountView],
                data: &[u8],
            ) -> ::pinocchio::ProgramResult {
                let discriminator = *data.first()
                    .ok_or(::pinocchio::error::ProgramError::InvalidInstructionData)?;
                match discriminator {
                    #(#process_arms)*
                    _ => Err(::pinocchio::error::ProgramError::InvalidInstructionData),
                }
            }
        }

        /// Program entrypoint
        #[inline]
        pub fn process_instruction(
            program_id: &::solana_address::Address,
            accounts: &[::pinocchio::AccountView],
            instruction_data: &[u8],
        ) -> ::pinocchio::ProgramResult {
            #enum_name::process(program_id, accounts, instruction_data)
        }

        #[cfg(not(feature = "no-entrypoint"))]
        ::pinocchio::entrypoint!(process_instruction);

        /// Get all instruction metadata for IDL generation.
        /// Returns a static slice of InstructionMeta for each instruction.
        #[cfg(feature = "idl")]
        pub const IDL_INSTRUCTIONS: &[::solzempic::InstructionMeta] = &[
            #(#idl_entries),*
        ];

        /// Shank-compatible instruction metadata module.
        /// Generated by SolzempicEntrypoint macro to provide IDL metadata
        /// when using expression discriminants (which ShankInstruction doesn't support).
        pub mod instruction_meta {
            #(#shank_instruction_metas)*
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for instruction impl blocks.
///
/// Transforms a regular impl block into `InstructionParams` and `Instruction<'a>`
/// trait implementations, enabling integration with the dispatch system.
///
/// # Three-Phase Pattern
///
/// Instructions follow a three-phase execution model:
///
/// | Phase | Method | Purpose |
/// |-------|--------|---------|
/// | 1 | `build` | Parse accounts, create instruction struct |
/// | 2 | `validate` | Check invariants, verify state |
/// | 3 | `execute` | Perform state mutations |
///
/// This separation enables clear responsibility boundaries and easier testing.
///
/// # Required Methods
///
/// All three methods must be implemented:
///
/// ```ignore
/// fn build(accounts: &'a [AccountInfo], params: &Params) -> Result<Self, ProgramError>
/// fn validate(&self, program_id: &Pubkey, params: &Params) -> ProgramResult
/// fn execute(&self, program_id: &Pubkey, params: &Params) -> ProgramResult
/// ```
///
/// # Generated Code
///
/// From a single impl block, generates:
///
/// 1. `impl InstructionParams for MyInstruction<'_>` - Associates params type
/// 2. `impl<'a> Instruction<'a> for MyInstruction<'a>` - Full instruction trait
///
/// The `Instruction::process` default method calls all three phases in order.
///
/// # Example
///
/// ```ignore
/// use solzempic::{instruction, AccountRefMut, Signer};
///
/// /// Parameters for the transfer instruction.
/// #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
/// #[repr(C)]
/// pub struct TransferParams {
///     pub amount: u64,
/// }
///
/// /// Transfer tokens between accounts.
/// pub struct Transfer<'a> {
///     from: AccountRefMut<'a, TokenAccount>,
///     to: AccountRefMut<'a, TokenAccount>,
///     authority: Signer<'a>,
/// }
///
/// #[instruction(TransferParams)]
/// impl<'a> Transfer<'a> {
///     fn build(accounts: &'a [AccountInfo], _params: &TransferParams) -> Result<Self, ProgramError> {
///         Ok(Self {
///             from: AccountRefMut::load(&accounts[0])?,
///             to: AccountRefMut::load(&accounts[1])?,
///             authority: Signer::wrap(&accounts[2])?,
///         })
///     }
///
///     fn validate(&self, _program_id: &Pubkey, params: &TransferParams) -> ProgramResult {
///         // Verify authority owns source account
///         if self.from.get().owner != *self.authority.key() {
///             return Err(ProgramError::InvalidAccountOwner);
///         }
///         // Verify sufficient balance
///         if self.from.get().amount() < params.amount {
///             return Err(ProgramError::InsufficientFunds);
///         }
///         Ok(())
///     }
///
///     fn execute(&self, _program_id: &Pubkey, params: &TransferParams) -> ProgramResult {
///         // Perform transfer via CPI...
///         Ok(())
///     }
/// }
/// ```
///
/// # Panics
///
/// Compile-time panics if:
/// - No params type provided in attribute (for impl blocks)
/// - Applied to non-struct/non-impl
#[proc_macro_attribute]
pub fn instruction(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Try to parse as struct first, then as impl block
    let item_clone = item.clone();

    if let Ok(input) = syn::parse::<ItemStruct>(item_clone) {
        // It's a struct - generate Shank account metadata
        return instruction_struct_impl(attr, input);
    }

    // Otherwise treat as impl block
    let params_type: syn::Path = syn::parse(attr)
        .expect("instruction macro on impl requires params type, e.g. #[instruction(MyParams)]");
    let input = parse_macro_input!(item as ItemImpl);

    // Extract the struct name from the impl
    let struct_type = &input.self_ty;

    // Extract struct name without lifetime for InstructionParams impl
    let struct_name = match struct_type.as_ref() {
        syn::Type::Path(type_path) => &type_path.path.segments.last().unwrap().ident,
        _ => panic!("instruction macro requires a struct type"),
    };

    // Extract the methods
    let methods: Vec<_> = input.items.iter().filter_map(|item| {
        if let ImplItem::Fn(method) = item {
            Some(method)
        } else {
            None
        }
    }).collect();

    let struct_name_str = struct_name.to_string();

    let expanded = quote! {
        impl ::solzempic::InstructionParams for #struct_name<'_> {
            type Params = #params_type;
        }

        impl<'a> ::solzempic::Instruction<'a> for #struct_name<'a> {
            #(#methods)*
        }

        impl #struct_name<'_> {
            /// Instruction name for IDL.
            pub const IDL_NAME: &'static str = #struct_name_str;

            /// Get params field metadata for IDL generation.
            pub const IDL_PARAMS: &'static [::solzempic::ParamField] = <#params_type as ::solzempic::ParamsMeta>::FIELDS;
        }
    };

    TokenStream::from(expanded)
}

/// Internal implementation for instruction struct
fn instruction_struct_impl(attr: TokenStream, input: ItemStruct) -> TokenStream {
    let struct_name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let generics = &input.generics;

    // Parse optional starting index from attribute (defaults to 0)
    let start_index: usize = if attr.is_empty() {
        0
    } else {
        syn::parse::<syn::LitInt>(attr)
            .map(|lit| lit.base10_parse::<usize>().unwrap_or(0))
            .unwrap_or(0)
    };

    let fields = match &input.fields {
        Fields::Named(fields_named) => &fields_named.named,
        _ => panic!("instruction macro on struct only supports named fields"),
    };

    // Analyze each field and determine account constraints
    let mut account_metas: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut shank_attr_strings: Vec<String> = Vec::new();
    let mut current_idx = start_index;

    for field in fields.iter() {
        let field_name = field.ident.as_ref().expect("Named field required");
        let field_name_str = field_name.to_string();
        let field_ty = &field.ty;

        let (is_signer, is_writable, is_program, expand_count) = analyze_field_type(field_ty);

        if expand_count > 1 {
            // Generate nested shard names: {field_name}_low_shard, {field_name}_current_shard, {field_name}_high_shard
            let shard_suffixes = ["low_shard", "current_shard", "high_shard"];
            for (i, suffix) in shard_suffixes.iter().enumerate() {
                let idx = current_idx + i;
                let nested_name = format!("{}_{}", field_name_str, suffix);
                account_metas.push(quote! {
                    ::solzempic::ShankAccountMeta {
                        index: #idx,
                        name: #nested_name,
                        is_signer: false,
                        is_writable: true,
                        is_program: false,
                    }
                });
                shank_attr_strings.push(format!("#[account({}, writable, name=\"{}\")]", idx, nested_name));
            }
            current_idx += expand_count;
        } else {
            let mut constraints = Vec::new();
            if is_writable { constraints.push("writable"); }
            if is_signer { constraints.push("signer"); }

            let constraints_str = if constraints.is_empty() {
                String::new()
            } else {
                format!(", {}", constraints.join(", "))
            };

            shank_attr_strings.push(format!("#[account({}{}, name=\"{}\")]", current_idx, constraints_str, field_name_str));

            account_metas.push(quote! {
                ::solzempic::ShankAccountMeta {
                    index: #current_idx,
                    name: #field_name_str,
                    is_signer: #is_signer,
                    is_writable: #is_writable,
                    is_program: #is_program,
                }
            });
            current_idx += 1;
        }
    }

    let num_accounts = account_metas.len();
    let shank_output = shank_attr_strings.join("\n    ");

    let field_defs = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_ty = &f.ty;
        let field_vis = &f.vis;
        let field_attrs = &f.attrs;
        quote! {
            #(#field_attrs)*
            #field_vis #field_name: #field_ty
        }
    });

    let expanded = quote! {
        #(#attrs)*
        #vis struct #struct_name #generics {
            #(#field_defs),*
        }

        impl #struct_name<'_> {
            pub const NUM_ACCOUNTS: usize = #num_accounts;

            pub const SHANK_ACCOUNTS: [::solzempic::ShankAccountMeta; #num_accounts] = [
                #(#account_metas),*
            ];

            pub fn shank_accounts() -> &'static str {
                #shank_output
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for account structs with automatic discriminator handling.
///
/// This macro transforms a simple struct definition into a zero-copy-safe
/// account type with all necessary traits and discriminator validation.
///
/// # What It Generates
///
/// From your struct definition, the macro produces:
///
/// | Generated | Purpose |
/// |-----------|---------|
/// | `#[repr(C)]` | Stable, predictable memory layout |
/// | `Clone`, `Copy` | Value semantics |
/// | `Pod`, `Zeroable` | Safe zero-copy casting via bytemuck |
/// | `discriminator` field | 8-byte type identifier (prepended) |
/// | `Loadable` impl | Zero-copy loading with validation |
///
/// # Account Layout
///
/// The discriminator is prepended to your fields:
///
/// ```text
/// Original:                    Generated:
/// struct Counter {             struct Counter {
///     owner: Pubkey,      →        discriminator: [u8; 8],  // Added
///     count: u64,                  owner: Pubkey,
/// }                                count: u64,
///                              }
/// ```
///
/// # Discriminator Values
///
/// Use unique discriminator values (0-255) for each account type in your
/// program. This prevents account type confusion attacks.
///
/// | Value | Recommendation |
/// |-------|----------------|
/// | 0 | Reserved (uninitialized) |
/// | 1-255 | Your account types |
///
/// # Required Attribute
///
/// The `#[account(discriminator = N)]` attribute is required:
///
/// ```ignore
/// #[derive(Account)]
/// #[account(discriminator = 1)]  // Required!
/// pub struct MyAccount { ... }
/// ```
///
/// # Field Requirements
///
/// All fields must be `Pod`-safe (no padding, alignment 1 or power-of-2):
///
/// | Safe Types | Unsafe Types |
/// |------------|--------------|
/// | `u8`, `u16`, `u32`, `u64`, `u128` | `bool` (use `u8`) |
/// | `i8`, `i16`, `i32`, `i64`, `i128` | `enum` (use `#[repr(u8)]`) |
/// | `[u8; N]`, `Pubkey` | `String`, `Vec<T>` |
/// | Other `Pod` structs | References, Box, Rc |
///
/// # Example
///
/// ```ignore
/// use solzempic::Account;
/// use pinocchio::pubkey::Pubkey;
///
/// /// A simple counter account.
/// #[derive(Account)]
/// #[account(discriminator = 1)]
/// pub struct Counter {
///     /// The authority who can increment.
///     pub authority: Pubkey,
///     /// Current count value.
///     pub count: u64,
/// }
///
/// /// User profile with multiple fields.
/// #[derive(Account)]
/// #[account(discriminator = 2)]
/// pub struct UserProfile {
///     pub owner: Pubkey,
///     pub created_at: i64,
///     pub points: u64,
///     pub level: u8,
///     pub _padding: [u8; 7],  // Explicit padding for alignment
/// }
/// ```
///
/// # Usage with AccountRef
///
/// ```ignore
/// fn increment(accounts: &[AccountInfo]) -> ProgramResult {
///     let counter = AccountRefMut::<Counter>::load(&accounts[0])?;
///
///     // Discriminator is automatically validated during load
///     counter.get_mut().count += 1;
///     Ok(())
/// }
/// ```
///
/// # Panics
///
/// Compile-time panics if:
/// - `#[account(discriminator = N)]` attribute is missing
/// - Applied to non-struct (enum, union)
/// - Struct has unnamed fields (tuple struct)
#[proc_macro_derive(Account, attributes(account))]
pub fn derive_account(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let vis = &input.vis;

    // Extract the discriminator value from #[account(discriminator = N)] attribute
    let discriminator = extract_discriminator(&input.attrs)
        .expect("Account derive requires #[account(discriminator = N)] attribute");

    // Get the struct fields
    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("Account derive only supports structs with named fields"),
        },
        _ => panic!("Account derive only supports structs"),
    };

    // Generate the new struct with discriminator field prepended
    let field_defs = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_ty = &f.ty;
        let field_vis = &f.vis;
        let attrs = &f.attrs;
        quote! {
            #(#attrs)*
            #field_vis #field_name: #field_ty
        }
    });

    let expanded = quote! {
        #[repr(C)]
        #[derive(Clone, Copy, ::bytemuck::Pod, ::bytemuck::Zeroable)]
        #[derive(::solzempic::shank::ShankAccount)]
        #vis struct #name {
            /// Account discriminator (8 bytes)
            pub discriminator: [u8; 8],
            #(#field_defs),*
        }

        impl #name {
            /// The discriminator value for this account type.
            pub const DISCRIMINATOR_VALUE: u8 = #discriminator;

            /// The discriminator as an 8-byte array.
            pub const DISCRIMINATOR_BYTES: [u8; 8] = [#discriminator, 0, 0, 0, 0, 0, 0, 0];

            /// Check if data has the correct discriminator.
            #[inline]
            pub fn check_discriminator(data: &[u8]) -> bool {
                !data.is_empty() && data[0] == #discriminator
            }
        }

        impl ::solzempic::Loadable for #name {
            const DISCRIMINATOR: u8 = #discriminator;
        }
    };

    TokenStream::from(expanded)
}

/// Analyzes a field type to determine Shank constraints.
/// Returns (is_signer, is_writable, is_program, expand_count)
fn analyze_field_type(ty: &Type) -> (bool, bool, bool, usize) {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                let type_name = segment.ident.to_string();
                match type_name.as_str() {
                    // Signer types
                    "Signer" => (true, false, false, 1),
                    "MutSigner" => (true, true, false, 1),  // signer + writable
                    "Payer" => (true, true, false, 1),      // payers are always signer + writable

                    // Writable account types
                    "AccountRefMut" => (false, true, false, 1),
                    "TokenAccountRefMut" => (false, true, false, 1),
                    "Writable" => (false, true, false, 1),

                    // Readonly account types
                    "AccountRef" => (false, false, false, 1),
                    "TokenAccountRef" => (false, false, false, 1),
                    "Mint" => (false, false, false, 1),
                    "Vault" => (false, false, false, 1),
                    "SolVault" => (false, false, false, 1),
                    "ValidatedAccount" => (false, false, false, 1),
                    "ReadOnly" => (false, false, false, 1),

                    // Writable specialized types
                    "Lut" => (false, true, false, 1),  // LUTs are typically created/modified

                    // Program types
                    "SystemProgram" => (false, false, true, 1),
                    "TokenProgram" => (false, false, true, 1),
                    "AtaProgram" => (false, false, true, 1),
                    "AltProgram" => (false, false, true, 1),
                    "Token2022Program" => (false, false, true, 1),

                    // Shard context expands to 3 accounts
                    "ShardRefContext" => (false, false, false, 3),  // read-only
                    "ShardRefMutContext" => (false, true, false, 3),  // writable

                    _ => (false, false, false, 1),
                }
            } else {
                (false, false, false, 1)
            }
        }
        Type::Reference(_) => {
            // &'a AccountView - default to readonly (use Writable<'a> for writable)
            (false, false, false, 1)
        }
        _ => (false, false, false, 1),
    }
}

/// Attribute macro for account structs.
///
/// Adds `#[repr(C)]`, `#[derive(Clone, Copy)]`, unsafe Pod/Zeroable impls, and optionally
/// `#[derive(ShankAccount)]` (when `shank` feature is enabled).
///
/// If a discriminator is provided, also generates `impl Loadable`.
///
/// Uses unsafe impl for Pod/Zeroable to support structs with manually-verified padding.
///
/// # Example
///
/// ```ignore
/// // Without discriminator (just Pod/Zeroable):
/// #[account]
/// pub struct Market {
///     pub discriminator: [u8; 8],
///     pub admin: Pubkey,
/// }
///
/// // With discriminator (also generates impl Loadable):
/// #[account(discriminator = AccountType::Market)]
/// pub struct Market {
///     pub discriminator: [u8; 8],
///     pub admin: Pubkey,
/// }
/// ```
#[proc_macro_attribute]
pub fn account(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let generics = &input.generics;

    // Parse discriminator from attribute if provided
    let discriminator_expr: Option<syn::Expr> = if attr.is_empty() {
        None
    } else {
        let attr_str = attr.to_string();
        // Parse "discriminator = <expr>"
        if let Some(eq_pos) = attr_str.find('=') {
            let expr_str = attr_str[eq_pos + 1..].trim();
            syn::parse_str(expr_str).ok()
        } else {
            None
        }
    };

    let fields = match &input.fields {
        Fields::Named(fields_named) => &fields_named.named,
        _ => panic!("account macro only supports structs with named fields"),
    };

    let field_defs = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_ty = &f.ty;
        let field_vis = &f.vis;
        let field_attrs = &f.attrs;
        quote! {
            #(#field_attrs)*
            #field_vis #field_name: #field_ty
        }
    });

    // Check if struct has a discriminator field
    let has_discriminator_field = fields.iter().any(|f| {
        f.ident.as_ref().map(|i| i == "discriminator").unwrap_or(false)
    });

    // Generate Loadable impl if discriminator provided
    let loadable_impl = discriminator_expr.clone().map(|disc| {
        let account_impl = if has_discriminator_field {
            quote! {
                impl ::solzempic::traits::Account for #name {
                    const DISCRIMINATOR: u8 = #disc as u8;
                    const LEN: usize = ::core::mem::size_of::<Self>();

                    #[inline]
                    fn discriminator(&self) -> &[u8; 8] {
                        &self.discriminator
                    }
                }
            }
        } else {
            quote! {}
        };

        quote! {
            impl ::solzempic::Loadable for #name {
                const DISCRIMINATOR: u8 = #disc as u8;
            }

            impl ::solzempic::Initializable for #name {}

            #account_impl
        }
    });

    // Generate field metadata for IDL
    let field_metas: Vec<_> = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().expect("named field").to_string();
        let field_type = type_to_string(&f.ty);
        quote! {
            ::solzempic::FieldMeta {
                name: #field_name,
                type_name: #field_type,
            }
        }
    }).collect();

    let name_str = name.to_string();

    // Generate AccountIdlMeta impl if discriminator is provided
    let idl_meta_impl = discriminator_expr.as_ref().map(|disc| {
        quote! {
            impl ::solzempic::AccountIdlMeta for #name {
                const NAME: &'static str = #name_str;
                const DISCRIMINATOR: u8 = #disc as u8;
                const FIELDS: &'static [::solzempic::FieldMeta] = &[
                    #(#field_metas),*
                ];
                const META: ::solzempic::AccountTypeMeta = ::solzempic::AccountTypeMeta {
                    name: Self::NAME,
                    discriminator: Self::DISCRIMINATOR,
                    fields: Self::FIELDS,
                };
            }

            // Auto-register with inventory when idl feature is enabled
            #[cfg(feature = "idl")]
            ::solzempic::inventory::submit! {
                &<#name as ::solzempic::AccountIdlMeta>::META
            }
        }
    });

    let expanded = quote! {
        #[repr(C)]
        #[derive(Clone, Copy)]
        #[derive(::solzempic::shank::ShankAccount)]
        #(#attrs)*
        #vis struct #name #generics {
            #(#field_defs),*
        }

        // Safety: Struct is #[repr(C)] - caller ensures no uninitialized padding
        unsafe impl ::bytemuck::Pod for #name {}
        unsafe impl ::bytemuck::Zeroable for #name {}

        #loadable_impl

        #idl_meta_impl
    };

    TokenStream::from(expanded)
}

/// Extract discriminator value from `#[account(discriminator = N)]` attribute.
///
/// Parses the attribute list looking for the `account` attribute with a
/// `discriminator` key-value pair. The value must be a u8 integer literal.
///
/// # Returns
///
/// - `Some(n)` if a valid discriminator attribute is found
/// - `None` if the attribute is missing or malformed
///
/// # Example Attribute Formats
///
/// ```ignore
/// #[account(discriminator = 1)]     // ✓ Valid
/// #[account(discriminator = 255)]   // ✓ Valid
/// #[account(discriminator = 256)]   // ✗ Overflow (not u8)
/// #[account(discriminator = "1")]   // ✗ String, not integer
/// ```
fn extract_discriminator(attrs: &[syn::Attribute]) -> Option<u8> {
    for attr in attrs {
        if attr.path().is_ident("account") {
            let nested = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated
            ).ok()?;

            for meta in nested {
                if let syn::Meta::NameValue(nv) = meta {
                    if nv.path.is_ident("discriminator") {
                        if let Expr::Lit(expr_lit) = &nv.value {
                            if let Lit::Int(lit_int) = &expr_lit.lit {
                                return lit_int.base10_parse::<u8>().ok();
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Attribute macro for instruction parameter structs.
///
/// Generates `impl InstructionParams` with field metadata for IDL generation.
/// Also adds `#[repr(C)]`, `#[derive(Clone, Copy)]`, and Pod/Zeroable impls.
///
/// # Example
///
/// ```ignore
/// #[params]
/// pub struct CancelClmmPositionParams {
///     pub order_id: u64,
///     pub side: u8,
///     pub _padding: [u8; 7],
/// }
///
/// // Generates:
/// // impl InstructionParams for CancelClmmPositionParams {
/// //     const FIELDS: &'static [ParamField] = &[
/// //         ParamField { name: "order_id", type_name: "u64" },
/// //         ParamField { name: "side", type_name: "u8" },
/// //         ParamField { name: "_padding", type_name: "[u8; 7]" },
/// //     ];
/// // }
/// ```
#[proc_macro_attribute]
pub fn params(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;

    // Handle unit structs (no fields)
    if matches!(&input.fields, syn::Fields::Unit) {
        let expanded = quote! {
            #[repr(C)]
            #[derive(Clone, Copy)]
            #(#attrs)*
            #vis struct #name;

            // Safety: Unit struct is always valid
            unsafe impl ::bytemuck::Pod for #name {}
            unsafe impl ::bytemuck::Zeroable for #name {}

            impl ::solzempic::ParamsMeta for #name {
                const FIELDS: &'static [::solzempic::ParamField] = &[];
            }
        };
        return TokenStream::from(expanded);
    }

    let fields = match &input.fields {
        syn::Fields::Named(f) => &f.named,
        _ => panic!("params macro only supports structs with named fields or unit structs"),
    };

    // Generate field definitions (preserve original)
    let field_defs = fields.iter().map(|f| {
        let field_name = &f.ident;
        let field_ty = &f.ty;
        let field_vis = &f.vis;
        let field_attrs = &f.attrs;
        quote! {
            #(#field_attrs)*
            #field_vis #field_name: #field_ty
        }
    });

    // Generate ParamField metadata
    let param_fields: Vec<_> = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().expect("Named field required");
        let field_name_str = field_name.to_string();
        let type_str = type_to_string(&f.ty);
        quote! {
            ::solzempic::ParamField {
                name: #field_name_str,
                type_name: #type_str,
            }
        }
    }).collect();

    let expanded = quote! {
        #[repr(C)]
        #[derive(Clone, Copy)]
        #(#attrs)*
        #vis struct #name {
            #(#field_defs),*
        }

        // Safety: Struct is #[repr(C)] with primitive fields
        unsafe impl ::bytemuck::Pod for #name {}
        unsafe impl ::bytemuck::Zeroable for #name {}

        impl ::solzempic::ParamsMeta for #name {
            const FIELDS: &'static [::solzempic::ParamField] = &[
                #(#param_fields),*
            ];
        }
    };

    TokenStream::from(expanded)
}

/// Convert a type to its string representation for IDL.
fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Path(tp) => {
            tp.path.segments.iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::")
        }
        Type::Array(arr) => {
            let elem = type_to_string(&arr.elem);
            let len = &arr.len;
            format!("[{}; {}]", elem, quote!(#len))
        }
        Type::Reference(r) => {
            let inner = type_to_string(&r.elem);
            if r.mutability.is_some() {
                format!("&mut {}", inner)
            } else {
                format!("&{}", inner)
            }
        }
        _ => quote!(#ty).to_string(),
    }
}