#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static SIGUIENTE_TEMPORAL: AtomicU64 = AtomicU64::new(0);

pub struct Temporal {
    pub ruta: PathBuf,
}

impl Temporal {
    pub fn nuevo(nombre: &str) -> Self {
        let id = SIGUIENTE_TEMPORAL.fetch_add(1, Ordering::Relaxed);
        let ruta = std::env::temp_dir().join(format!(
            "quetzal_cli_{nombre}_{}_{}",
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&ruta);
        std::fs::create_dir_all(&ruta).expect("crear directorio temporal");
        Self { ruta }
    }

    pub fn proyecto(&self, principal: &str) -> PathBuf {
        let entrada = self.ruta.join("principal.qz");
        std::fs::write(&entrada, principal).expect("escribir programa");
        std::fs::write(
            self.ruta.join("quetzal.json"),
            r#"{
  "version": "0.1.0",
  "aplicacion": "prueba_cli",
  "entrada": "principal.qz",
  "tipo": "aplicacion",
  "quetzal": "0.0.2",
  "dependencias": {},
  "permisos": {}
}"#,
        )
        .expect("escribir manifiesto");
        entrada
    }
}

impl Drop for Temporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.ruta);
    }
}

pub fn quetzal() -> Command {
    Command::new(env!("CARGO_BIN_EXE_quetzal"))
}

pub fn ejecutar(command: &mut Command, timeout: Duration) -> Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("iniciar quetzal");
    let limite = Instant::now() + timeout;
    loop {
        if child.try_wait().expect("consultar proceso").is_some() {
            return child.wait_with_output().expect("capturar salida");
        }
        if Instant::now() >= limite {
            let _ = child.kill();
            let salida = child.wait_with_output().expect("recoger proceso detenido");
            panic!(
                "quetzal excedió {timeout:?}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&salida.stdout),
                String::from_utf8_lossy(&salida.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub fn afirmar_salida(salida: &Output, esperada: &[u8]) {
    assert_eq!(
        salida.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
    assert_eq!(salida.stdout, esperada);
    assert!(
        salida.stderr.is_empty(),
        "stderr: {}",
        String::from_utf8_lossy(&salida.stderr)
    );
}

pub fn ejecutar_en(directorio: &Path, args: &[&str]) -> Output {
    let mut comando = quetzal();
    comando.current_dir(directorio).args(args);
    ejecutar(&mut comando, Duration::from_secs(10))
}
