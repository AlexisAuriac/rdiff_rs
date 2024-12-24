use std::{
    fs::{remove_file, File, OpenOptions},
    io::{self, stdin, stdout, BufReader, BufWriter, Read, Stdin, Stdout, Write},
    path::PathBuf,
};

use rdiff::{
    buf_reader_with_retry::BufReaderWithRetry,
    delta::delta as io_delta,
    error::Error,
    patch::patch as io_patch,
    signature::{read_signature, SignatureOptions},
};

// largely copied from the library whole.rs
// modified to accept "-" parameters (use stdin/stdout for input/output)

enum FileOrStdin {
    Stdin(Stdin),
    File(File),
}

impl FileOrStdin {
    pub fn new(p: &str) -> Result<FileOrStdin, Error> {
        if p == "-" {
            Ok(FileOrStdin::Stdin(stdin()))
        } else {
            let f = OpenOptions::new().read(true).open(p)?;
            Ok(FileOrStdin::File(f))
        }
    }
}

impl Read for FileOrStdin {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            FileOrStdin::Stdin(stdin) => stdin.read(buf),
            FileOrStdin::File(file) => file.read(buf),
        }
    }
}

enum FileOrStdout {
    Stdout(Stdout),
    File { f: File, p: PathBuf },
}

impl FileOrStdout {
    pub fn new(p: &str, force: bool) -> Result<FileOrStdout, Error> {
        if p == "-" {
            Ok(FileOrStdout::Stdout(stdout()))
        } else {
            let f = OpenOptions::new()
                .create(force)
                .create_new(!force)
                .truncate(true)
                .write(true)
                .open(p)?;
            Ok(FileOrStdout::File {
                f,
                p: PathBuf::from(p),
            })
        }
    }

    pub fn remove(self) -> Result<(), io::Error> {
        match self {
            FileOrStdout::Stdout(_) => Ok(()),
            FileOrStdout::File { f, p } => {
                drop(f);
                remove_file(p)
            }
        }
    }
}

impl Write for FileOrStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            FileOrStdout::Stdout(stdout) => stdout.write(buf),
            FileOrStdout::File { f, .. } => f.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            FileOrStdout::Stdout(stdout) => stdout.flush(),
            FileOrStdout::File { f, .. } => f.sync_data(),
        }
    }
}

pub fn signature_opts(
    basis: &str,
    sig_file: &str,
    mut opts: SignatureOptions,
    force: bool,
) -> Result<(), Error> {
    let in_file = FileOrStdin::new(basis)?;
    let mut out_file = FileOrStdout::new(sig_file, force)?;

    match &in_file {
        FileOrStdin::File(file) => {
            let input_size = file.metadata()?.len();
            opts.input_size(input_size as usize);
        }
        FileOrStdin::Stdin(_) => (),
    }

    let res = opts.signature(
        &mut BufReaderWithRetry::new(in_file),
        &mut BufWriter::new(&mut out_file),
    );
    if let Err(err) = res {
        out_file
            .remove()
            .unwrap_or_else(|err| eprintln!("{:?}: {}", sig_file, err));
        return Err(err);
    }

    let res = out_file.flush();
    if let Err(err) = res {
        out_file
            .remove()
            .unwrap_or_else(|err| eprintln!("{:?}: {}", sig_file, err));
        return Err(err.into());
    }

    Ok(())
}

pub fn delta(
    sig_file: &str,
    new_file: &str,
    delta_path: &str,
    force: bool,
    input_size: Option<usize>,
) -> Result<(), Error> {
    let sig = {
        let mut sig_file = FileOrStdin::new(sig_file)?;
        let input_size = match (input_size, &sig_file) {
            (Some(size), _) => Some(size),
            (None, FileOrStdin::File(file)) => Some(file.metadata()?.len() as usize),
            (None, FileOrStdin::Stdin(_)) => None,
        };

        read_signature(&mut sig_file, input_size)?
    };

    let new_file = FileOrStdin::new(new_file)?;
    let mut delta_file = FileOrStdout::new(delta_path, force)?;

    let res = io_delta(&sig, &mut BufReader::new(new_file), &mut delta_file);
    if let Err(err) = res {
        delta_file
            .remove()
            .unwrap_or_else(|err| eprintln!("{:?}: {}", delta_path, err));
        return Err(err);
    }

    let res = delta_file.flush();
    if let Err(err) = res {
        delta_file
            .remove()
            .unwrap_or_else(|err| eprintln!("{:?}: {}", delta_path, err));
        return Err(err.into());
    }

    Ok(())
}

pub fn patch(basis: &str, delta_file: &str, new_path: &str, force: bool) -> Result<(), Error> {
    let mut old_file = OpenOptions::new().read(true).open(basis)?;
    let mut delta_file = FileOrStdin::new(delta_file)?;
    let mut new_file = FileOrStdout::new(new_path, force)?;

    let res = io_patch(&mut old_file, &mut delta_file, &mut new_file);
    if let Err(err) = res {
        new_file
            .remove()
            .unwrap_or_else(|err| eprintln!("{:?}: {}", new_path, err));
        return Err(err);
    }

    let res = new_file.flush();
    if let Err(err) = res {
        new_file
            .remove()
            .unwrap_or_else(|err| eprintln!("{:?}: {}", new_path, err));
        return Err(err.into());
    }

    Ok(())
}
