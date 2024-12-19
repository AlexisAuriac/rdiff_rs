use std::io::{Read, Write};

use rdiff::{error::Error, signature::Signature, signature::SignatureOptions};

#[cfg(not(test))]
#[inline]
pub fn read_signature<I>(input: &mut I) -> Result<Signature, Error>
where
    I: Read,
{
    use rdiff::signature::read_signature;
    read_signature(input)
}

#[cfg(not(test))]
#[inline]
pub fn signature<I, O>(opts: SignatureOptions, input: &mut I, output: &mut O) -> Result<(), Error>
where
    I: Read,
    O: Write,
{
    opts.signature(input, output)
}

#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
type SignatureFn =
    Box<dyn Fn(SignatureOptions, &mut dyn Read, &mut dyn Write) -> Result<(), Error>>;

#[cfg(test)]
thread_local! {
    pub static READ_SIGNATURE_OUTPUT: Mutex<Option<Result<Signature, Error>>> = const { Mutex::new(None) };
    pub static SIGNATURE_FN: Mutex<Option<SignatureFn>> = const { Mutex::new(None) };
}

#[cfg(test)]
pub fn signature<I, O>(opts: SignatureOptions, input: &mut I, output: &mut O) -> Result<(), Error>
where
    I: Read,
    O: Write,
{
    println!("here");
    let f = SIGNATURE_FN.with(|static_sig| {
        static_sig
            .lock()
            .unwrap()
            .take()
            .expect("SIGNATURE_FN was not initialized before call")
    });

    f(opts, input, output)
}

#[cfg(test)]
pub fn set_signature_fn(f: SignatureFn) {
    SIGNATURE_FN.with(|static_sig| {
        let _ = static_sig.lock().unwrap().replace(f);
    });
}

#[cfg(test)]
pub fn set_next_read_signature_output(res: Result<Signature, Error>) {
    READ_SIGNATURE_OUTPUT.with(|static_sig| {
        let _ = static_sig.lock().unwrap().replace(res).unwrap();
    });
}

#[cfg(test)]
pub fn read_signature<I>(_input: &mut I) -> Result<Signature, Error>
where
    I: Read,
{
    READ_SIGNATURE_OUTPUT.with(|static_sig| {
        static_sig
            .lock()
            .unwrap()
            .take()
            .expect("READ_SIGNATURE_INPUT was not initialized before call")
    })
}
