use std::process::Stdio;

use std::io;
use std::string::FromUtf8Error;

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::select;

#[derive(Error, Debug)]
pub enum JShellError {
    #[error("jshell output contained a non utf-8 character")]
    UnexpectedOutputError(#[from] FromUtf8Error),

    #[error("jshell io error: {0:?}")]
    IOError(#[from] io::Error),

    #[error("failed to spawn jshell: {0:?}")]
    SpawnError(io::Error),

    #[error("jshell closed unexpectedly")]
    ClosedError,
}

pub struct JShell {
    // keep reference to jshell, as it is killed when dropped
    _instance: Child,
    out: Lines<BufReader<ChildStdout>>,
    err: BufReader<ChildStderr>,
    input: ChildStdin,
}

impl JShell {
    pub async fn new() -> Result<Self, JShellError> {
        let mut cmd = Command::new("jshell")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| JShellError::SpawnError(e))?;

        let stdin = cmd
            .stdin
            .take()
            .expect("failed to get stdin of jshell. this shouldnt happen");

        let stdout = cmd
            .stdout
            .take()
            .expect("failed to get stdout of jshell. this shouldnt happen");
        let bufout = BufReader::new(stdout).lines();

        let stderr = cmd
            .stderr
            .take()
            .expect("failed to get stderr of jshell. this shouldnt happen");
        let buferr = BufReader::new(stderr);

        let mut jshell = JShell {
            _instance: cmd,
            out: bufout,
            err: buferr,
            input: stdin,
        };

        // set prompt to null byte to detect end of output more easily
        jshell.input("/set mode mymode normal -command\n").await?;
        jshell
            .input("/set prompt mymode \"\\0\" \"   continue$ \"\n")
            .await?;
        jshell.input("/set feedback mymode\n").await?;

        // discard welcome message
        loop {
            let out = jshell.read_line().await?;
            if out.1 {
                break;
            }
        }

        Ok(jshell)
    }

    pub async fn read_line(&mut self) -> Result<(String, bool), JShellError> {
        let mut err_vec = Vec::new();

        let out = select! {
            line = self.out.next_line() => {
                // next_line returns a result, which is error checked normally
                // this result contains an option, which is none if the stream is closed
                // a closed stream is mapped to a jshell closed error
                (line?.ok_or(JShellError::ClosedError)?,false)
            }
            _ = self.err.read_until(b'\x00', &mut err_vec) => {
                if err_vec.last().is_none(){
                    return Err(JShellError::ClosedError)
                } else{
                    (String::from_utf8(err_vec)?,true)
                }
            }
        };

        Ok(out)
    }

    pub async fn read_output(&mut self) -> Result<String, JShellError> {
        let mut output = String::new();
        loop {
            let out = self.read_line().await?;
            if out.1 {
                break;
            } else {
                output.push_str(&out.0);
            }
        }
        Ok(output)
    }

    pub async fn input(&mut self, stmt: &str) -> Result<(), JShellError> {
        Ok(self.input.write_all(stmt.as_bytes()).await?)
    }
}
