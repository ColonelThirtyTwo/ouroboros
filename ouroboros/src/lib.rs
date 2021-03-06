//! A crate for creating safe self-referencing structs.
//!
//! See the documentation for [`#[self_referencing]`](self_referencing) to get started.
//! See the documentation of [`ouroboros_examples`](https://docs.rs/ouroboros_examples) for
//! sample documentation of structs which have had the macro applied to them.

#![allow(clippy::needless_doctest_main)]

/// This macro is used to turn a regular struct into a self-referencing one. An example:
/// ```rust
/// use ouroboros::self_referencing;
///
/// #[self_referencing]
/// struct MyStruct {
///     int_data: Box<i32>,
///     float_data: Box<f32>,
///     #[borrows(int_data)]
///     int_reference: &'this i32,
///     #[borrows(mut float_data)]
///     float_reference: &'this mut f32,
/// }
///
/// fn main() {
///     let mut my_value = MyStructBuilder {
///         int_data: Box::new(42),
///         float_data: Box::new(3.14),
///         int_reference_builder: |int_data: &i32| int_data,
///         float_reference_builder: |float_data: &mut f32| float_data,
///     }.build();
/// 
///     // Prints 42
///     println!("{:?}", my_value.with_int_data_contents(|int_data| *int_data));
///     // Prints 3.14
///     println!("{:?}", my_value.with_float_reference(|float_reference| **float_reference));
///     // Sets the value of float_data to 84.0
///     my_value.with_mut(|fields| {
///         **fields.float_reference = (**fields.int_reference as f32) * 2.0;
///     });
///
///     // We can hold on to this reference...
///     let int_ref = my_value.with_int_reference(|int_ref| *int_ref);
///     println!("{:?}", *int_ref);
///     // As long as the struct is still alive.
///     drop(my_value);
///     // This will cause an error!
///     // println!("{:?}", *int_ref);
/// }
/// ```
/// To explain the features and limitations of this crate, some definitions are necessary:
/// # Definitions
/// - **immutably borrowed field**: a field which is immutably borrowed by at least one other field.
/// - **mutably borrowed field**: a field which is mutably borrowed by exactly one other field.
/// - **self-referencing field**: a field which borrows at least one other field.
/// - **head field**: a field which does not borrow any other fields, I.E. not self-referencing.
/// - **tail field**: a field which is not borrowed by any other fields.
///
/// To make a self-referencing struct, you must write a struct definition and place
/// `#[self_referencing]` on top. For every field that borrows other fields, you must place
/// `#[borrows()]` on top and place inside the parenthesis a list of fields that it borrows. Mut can
/// be prefixed to indicate that a mutable borrow is required. For example,
/// `#[borrows(a, b, mut c)]` indicates that the first two fields need to be borrowed immutably and
/// the third needs to be borrowed mutably.
/// # You must comply with these limitations
/// - Fields must be declared before the first time they are borrowed.
/// - Normal borrowing rules apply, E.G. a field cannot be borrowed mutably twice.
/// - Fields that are borrowed must be of a data type that implement
///   [`StableDeref`](https://docs.rs/stable_deref_trait/1.2.0/stable_deref_trait/trait.StableDeref.html).
///   Normally this just means `Box<T>`.
///
/// Violating them will result in a nice error message directly pointing out the violated rule.
/// # Flexibility of this crate
/// The example above uses plain references as the self-referencing part of the struct, but you can
/// use anything that is dependent on lifetimes of objects inside the struct. For example, you could
/// do something like this:
/// ```rust
/// use ouroboros::self_referencing;
///
/// pub struct ComplexData<'a, 'b> {
///     aref: &'a i32,
///     bref: &'b mut i32,
///     number: i32,
/// }
///
/// impl<'a, 'b> ComplexData<'a, 'b> {
///     fn new(aref: &'a i32, bref: &'b mut i32, number: i32) -> Self {
///         Self { aref, bref, number }
///     }
///
///     /// Copies the value aref points to into what bref points to.
///     fn transfer(&mut self) {
///         *self.bref = *self.aref;
///     }
///
///     /// Prints the value bref points to.
///     fn print_bref(&self) {
///         println!("{}", *self.bref);
///     }
/// }
///
/// fn main() {
///     #[self_referencing]
///     struct DataStorage {
///         immutable: Box<i32>,
///         mutable: Box<i32>,
///         #[borrows(immutable, mut mutable)]
///         complex_data: ComplexData<'this, 'this>,
///     }
///
///     let mut data_storage = DataStorageBuilder {
///         immutable: Box::new(10),
///         mutable: Box::new(20),
///         complex_data_builder: |i: &i32, m: &mut i32| ComplexData::new(i, m, 12345),
///     }.build();
///     data_storage.with_complex_data_mut(|data| {
///         // Copies the value in immutable into mutable.
///         data.transfer();
///         // Prints 10
///         data.print_bref();
///     });
/// }
/// ```
/// # Using `chain_hack`
/// Unfortunately, as of September 2020, Rust has a
/// [known limitation in its type checker](https://users.rust-lang.org/t/why-does-this-not-compile-box-t-target-t/49027/7?u=aaaaa)
/// which prevents chained references from working (I.E. structs where field C references field B
/// which references field A.) To counteract this problem, you can use
/// `#[self_referencing(chain_hack)]` to allow creating these kinds of structs at the cost of
/// additional restrictions and possible loss of clarity in some error messages. The main limitation
/// is that all fields that are borrowed must be of type `Box<T>`. A nice error message will be
/// generated if you use a different type. There should be no other limitations, but some
/// configurations may produce strange compiler errors. If you find such a configuration, please
/// open an issue on the [Github repository](https://github.com/joshua-maros/ouroboros/issues).
/// You can view a documented example of a struct which uses `chain_hack` [here](https://docs.rs/ouroboros_examples/latest/ouroboros_examples/struct.ChainHack.html).
/// # What does the macro generate?
/// The `#[self_referencing]` struct will replace your definition with an unsafe self-referencing
/// struct with a safe public interface. Many functions will be generated depending on your original
/// struct definition. Documentation is generated for all items, so building documentation for
/// your project allows accessing detailed information about available functions. Using 
/// `#[self_referencing(no_doc)]` will hide the generated items from documentation if it is becoming 
/// too cluttered. The following is an overview of what is generated:
/// ### `MyStruct::new(fields...) -> MyStruct`
/// A basic constructor. It accepts values for each field in the order you declared them in. For
/// **head fields**, you only need to pass in what value it should have and it will be moved in
/// to the output. For **self-referencing fields**, you must provide a function or closure which creates
/// the value based on the values it borrows. A field using the earlier example of
/// `#[borrow(a, b, mut c)]` would require a function typed as
/// `FnOnce(a: &_, b: &_, c: &mut _) -> _`.
/// ### `MyStructBuilder`
/// This is the preferred way to create a new instance of your struct. It is similar to using the
/// `MyStruct { a, b, c, d }` syntax instead of `MyStruct::new(a, b, c, d)`. It contains one field
/// for every argument in the actual constructor. **Head fields** have the same name that you
/// originally defined them with. **self-referencing fields** are suffixed with `_builder` since you need
/// to provide a function instead of a value. Calling `.build()` on an instance of `MyStructBuilder`
/// will convert it to an instance of `MyStruct`.
/// ### `MyStruct::try_new<E>(fields...) -> Result<MyStruct, E>`
/// Similar to the regular `new()` function, except the functions wich create values for all
/// **self-referencing fields** can return `Result<>`s. If any of those are `Err`s, that error will be
/// returned instead of an instance of `MyStruct`. The preferred way to use this function is through
/// `MyStructTryBuilder` and its `try_build()` function.
/// ### `MyStruct::try_new_or_recover<E>(fields...) -> Result<MyStruct, (E, Heads)>`
/// Similar to the `try_new()` function, except that all the **head fields** are returned along side
/// the original error in case of an error. The preferred way to use this function is through
/// `MyStructTryBuilder` and its `try_build_or_recover()` function.
/// ### `MyStruct::with_FIELD<R>(&self, user: FnOnce(field: &FieldType) -> R) -> R`
/// This function is generated for every **tail field** in your struct. It allows safely accessing
/// a reference to that value. The function generates the reference and passes it to `user`. You
/// can do anything you want with the reference, it is constructed to not outlive the struct.
/// ### `MyStruct::with_FIELD_mut<R>(&mut self, user: FnOnce(field: &mut FieldType) -> R) -> R`
/// This function is generated for every **tail field** in your struct. It is the mutable version
/// of `with_FIELD`.
/// ### `MyStruct::with_FIELD_contents<R>(&self, user: FnOnce(data: &<FieldType as Deref>::Target) -> R) -> R`
/// This function is generated for every **immutably borrowed field** In your struct. It allows
/// accessing the contents of that field. It is similar to `with_FIELD` except that it provides
/// a reference to the field's content, not the field itself. E.G. a field of type `Box<i32>` would
/// cause this function to provide a reference of type `&i32`. There is no mutable version of this
/// function because if a field is already borrowed, it cannot be mutably borrowed safely.
/// ### `MyStruct::with<R>(&self, user: FnOnce(fields: AllFields) -> R) -> R`
/// Allows borrowing all **tail and immutably-borrowed fields** at once. Functions similarly to
/// `with_FIELD`.
/// ### `MyStruct::with_mut<R>(&self, user: FnOnce(fields: AllFields) -> R) -> R`
/// Allows mutably borrowing all **tail fields** at once. Functions similarly to `with_FIELD_mut`.
/// ### `MyStruct::into_heads(self) -> Heads`
/// Drops all self-referencing fields and returns a struct containing all **head fields**.
pub use ouroboros_macro::self_referencing;

#[doc(hidden)]
pub mod macro_help {
    use stable_deref_trait::StableDeref;
    use std::ops::DerefMut;

    /// Converts a reference to an object implementing Deref to a static reference to the data it
    /// Derefs to. This is obviously unsafe because the compiler can no longer guarantee that the
    /// data outlives the reference. This function is templated to only work for containers that
    /// implement StableDeref, E.G. Box and Rc. The intent is that the data that is being pointed
    /// to will never move as long as the container itself is not dropped. It is up to the consumer
    /// to get rid of the reference before the container is dropped. The + 'static ensures that
    /// whatever we are referring to will remain valid indefinitely, that there are no limitations
    /// on how long the pointer itself can live.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that the returned reference is not used after the originally passed
    /// reference would become invalid.
    pub unsafe fn stable_deref_and_strip_lifetime<T: StableDeref + 'static>(
        data: &T,
    ) -> &'static T::Target {
        &*((&**data) as *const _)
    }

    /// Like stable_deref_and_strip_lifetime, but for mutable references.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that the returned reference is not used after the originally passed
    /// reference would become invalid.
    pub unsafe fn stable_deref_and_strip_lifetime_mut<T: StableDeref + DerefMut + 'static>(
        data: &mut T,
    ) -> &'static mut T::Target {
        &mut *((&mut **data) as *mut _)
    }
}
