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
//! - `ShardRefContext<'a, T>` - Type alias for shard triplets
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
use syn::{parse_macro_input, Data, DeriveInput, Fields, Expr, Lit, ItemImpl, ImplItem};

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
/// | `ShardRefContext<'a, T>` | `type` | Shard triplet context alias |
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

    // Collect variant info (name and discriminator value)
    let variant_info: Vec<_> = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let discriminant = variant.discriminant.as_ref()
            .expect("SolzempicEntrypoint requires explicit discriminant values");
        let disc_expr = &discriminant.1;
        (variant_name, disc_expr)
    }).collect();

    // Generate TryFrom<u8> match arms
    let try_from_arms = variant_info.iter().map(|(name, disc)| {
        quote! { #disc => Ok(#enum_name::#name), }
    });

    // Generate dispatch match arms (for backward compat)
    let dispatch_arms = variant_info.iter().map(|(name, _)| {
        quote! {
            #enum_name::#name => <#name<'_> as ::solzempic::Instruction<'_>>::process(program_id, accounts, data),
        }
    });

    // Generate process match arms (direct discriminator to handler)
    let process_arms = variant_info.iter().map(|(name, disc)| {
        quote! {
            #disc => <#name<'_> as ::solzempic::Instruction<'_>>::process(program_id, accounts, &data[1..]),
        }
    });

    // Generate variant definitions for the enum
    let variant_defs = variants.iter().map(|v| {
        let name = &v.ident;
        let disc = &v.discriminant.as_ref().unwrap().1;
        quote! { #name = #disc }
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

        /// Context for sharded data structures.
        pub type ShardRefContext<'a, T> = ::solzempic::ShardRefContext<'a, T, Solzempic>;

        /// Returns the program ID.
        #[inline]
        pub fn id() -> &'static ::solana_address::Address {
            &ID
        }

        #(#attrs)*
        #[repr(u8)]
        #vis enum #enum_name {
            #(#variant_defs),*
        }

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
/// - No params type provided in attribute
/// - Applied to a non-struct impl block
#[proc_macro_attribute]
pub fn instruction(attr: TokenStream, item: TokenStream) -> TokenStream {
    let params_type: syn::Path = syn::parse(attr)
        .expect("instruction macro requires params type, e.g. #[instruction(MyParams)]");
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

    let expanded = quote! {
        impl ::solzempic::InstructionParams for #struct_name<'_> {
            type Params = #params_type;
        }

        impl<'a> ::solzempic::Instruction<'a> for #struct_name<'a> {
            #(#methods)*
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
        #vis struct #name {
            pub discriminator: [u8; 8],
            #(#field_defs),*
        }

        impl ::braid_types::Loadable for #name {
            const DISCRIMINATOR: ::braid_types::AccountType =
                unsafe { ::core::mem::transmute::<u8, ::braid_types::AccountType>(#discriminator) };
        }
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