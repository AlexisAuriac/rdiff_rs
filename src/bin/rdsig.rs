use std::alloc::{GlobalAlloc, Layout, System};
use std::env::args;
use std::fmt::{self, Display, Formatter};

use rdiff::signature::read_signature_file2;
use rdiff::{error::Error, signature::read_signature_file};

static mut MEM_ALLOCATED: usize = 0;
static mut ALLOC_CALLS: usize = 0;

#[derive(Debug, Clone)]
struct MemReport {
    mem_allocated: usize,
    alloc_calls: usize,
}

fn format_mem(mem: usize) -> String {
    if mem > 1_000_000_000 {
        format!("{}Gb", mem / 1_000_000_000)
    } else if mem > 1_000_000 {
        format!("{}Mb", mem / 1_000_000)
    } else if mem > 1_000 {
        format!("{}kb", mem / 1_000)
    } else {
        format!("{}b", mem)
    }
}

impl Display for MemReport {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f, "mem used   : {}", format_mem(self.mem_allocated))?;
        write!(f, "alloc calls: {}", self.alloc_calls)?;

        Ok(())
    }
}

struct MyAllocator;

unsafe impl GlobalAlloc for MyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        MEM_ALLOCATED += layout.size();
        ALLOC_CALLS += 1;
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: MyAllocator = MyAllocator;

fn get_mem_used<T>(f: impl FnOnce() -> T) -> (T, MemReport) {
    unsafe {
        MEM_ALLOCATED = 0;
        ALLOC_CALLS = 0;
        let x = f();
        (
            x,
            MemReport {
                mem_allocated: MEM_ALLOCATED,
                alloc_calls: ALLOC_CALLS,
            },
        )
    }
}

fn cmp() -> Result<(), Error> {
    let sig_p = args().nth(1).unwrap();

    let (sig1, mem) = get_mem_used(|| read_signature_file(&sig_p).unwrap());
    println!("{}", mem);

    println!("==============");

    let (sig2, mem) = get_mem_used(|| read_signature_file2(&sig_p).unwrap());
    println!("{}", mem);

    for (i, sum1) in sig1.strong_sigs.iter().enumerate() {
        let sum2 = sig2.strong_sigs.at(i);

        assert_eq!(sum1.as_slice(), sum2);
    }

    Ok(())
}

fn sig1() -> Result<(), Error> {
    let sig_p = args().nth(1).unwrap();
    read_signature_file(&sig_p)?;
    Ok(())
}

fn sig2() -> Result<(), Error> {
    let sig_p = args().nth(1).unwrap();
    read_signature_file2(&sig_p)?;
    Ok(())
}

fn main() -> Result<(), Error> {
    // cmp()
    // sig1()
    sig2()
}
