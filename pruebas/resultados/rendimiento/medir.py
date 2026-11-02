#!/usr/bin/env python3
import csv
import json
import os
import platform
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

RAIZ = Path(__file__).resolve().parents[3]
SALIDA = Path(__file__).resolve().parent
PROGRAMAS = RAIZ / "pruebas" / "compatibilidad" / "programas"
BINARIOS = {
    "automatico": Path("/tmp/quetzal-actual-con-jit"),
    "sin_jit": Path("/tmp/quetzal-actual-con-jit"),
    "sin_feature": Path("/tmp/quetzal-actual-sin-feature"),
    "base_003": Path("/tmp/quetzal-linea-base-647cfe7"),
    "v002": Path("/tmp/quetzal-v002"),
}
ESCENARIOS = {
    "corto": 31,
    "pequenas": 31,
    "bucles_cortos": 31,
    "repetidas": 7,
    "caliente": 7,
    "no_elegible": 7,
}


def ejecutar(binario, programa, banderas=()):
    inicio = time.perf_counter_ns()
    resultado = subprocess.run(
        [str(binario), "ejecutar", *banderas, str(programa)],
        cwd=RAIZ,
        capture_output=True,
        timeout=30,
        check=False,
    )
    return (time.perf_counter_ns() - inicio) / 1e6, resultado


def medir_escenario(modo, escenario, iteraciones):
    binario = BINARIOS[modo]
    banderas = ("--sin-jit",) if modo == "sin_jit" else ()
    programa = PROGRAMAS / f"{escenario}.qz"
    esperado = None
    for _ in range(2):
        _, resultado = ejecutar(binario, programa, banderas)
        if resultado.returncode != 0:
            raise RuntimeError(f"{modo}/{escenario}: {resultado.stderr.decode(errors='replace')}")
        esperado = resultado.stdout
    tiempos = []
    for _ in range(iteraciones):
        transcurrido, resultado = ejecutar(binario, programa, banderas)
        if resultado.returncode != 0 or resultado.stdout != esperado or resultado.stderr:
            raise RuntimeError(f"salida divergente en {modo}/{escenario}")
        tiempos.append(transcurrido)
    return {
        "muestras": iteraciones,
        "mediana_ms": statistics.median(tiempos),
        "desviacion_ms": statistics.stdev(tiempos) if len(tiempos) > 1 else 0.0,
        "minimo_ms": min(tiempos),
        "maximo_ms": max(tiempos),
        "stdout": esperado.decode(errors="replace"),
    }


def medir_cache(modo):
    binario = BINARIOS[modo]
    banderas = ("--sin-jit",) if modo == "sin_jit" else ()
    with tempfile.TemporaryDirectory(prefix="quetzal_rendimiento_cache_") as temporal:
        raiz = Path(temporal)
        shutil.copy(PROGRAMAS / "corto.qz", raiz / "principal.qz")
        (raiz / "quetzal.json").write_text(json.dumps({
            "version": "0.1.0",
            "aplicacion": "medicion",
            "entrada": "principal.qz",
            "tipo": "aplicacion",
            "quetzal": "0.0.2",
            "dependencias": {},
            "permisos": {},
        }))
        inicio = time.perf_counter_ns()
        frio = subprocess.run([str(binario), "ejecutar", *banderas], cwd=raiz, capture_output=True, timeout=30)
        frio_ms = (time.perf_counter_ns() - inicio) / 1e6
        if frio.returncode != 0:
            raise RuntimeError(f"caché fría {modo}: {frio.stderr.decode(errors='replace')}")
        tiempos = []
        for _ in range(15):
            inicio = time.perf_counter_ns()
            caliente = subprocess.run([str(binario), "ejecutar", *banderas], cwd=raiz, capture_output=True, timeout=30)
            tiempos.append((time.perf_counter_ns() - inicio) / 1e6)
            if caliente.returncode != 0 or caliente.stdout != frio.stdout or caliente.stderr:
                raise RuntimeError(f"caché caliente divergente {modo}")
        return {"fria_ms": frio_ms, "caliente_mediana_ms": statistics.median(tiempos), "muestras_caliente": len(tiempos)}


def rss_maximo(modo, escenario):
    binario = BINARIOS[modo]
    banderas = ("--sin-jit",) if modo == "sin_jit" else ()
    argumentos = [str(binario), "ejecutar", *banderas, str(PROGRAMAS / f"{escenario}.qz")]
    pid = os.fork()
    if pid == 0:
        try:
            nulo = os.open(os.devnull, os.O_RDWR)
            os.dup2(nulo, 1)
            os.dup2(nulo, 2)
            os.chdir(RAIZ)
            os.execv(str(binario), argumentos)
        finally:
            os._exit(127)
    _, estado, uso = os.wait4(pid, 0)
    if not os.WIFEXITED(estado) or os.WEXITSTATUS(estado) != 0:
        raise RuntimeError(f"RSS {modo}/{escenario}: estado {estado}")
    return uso.ru_maxrss


def main():
    ausentes = [str(ruta) for ruta in BINARIOS.values() if not ruta.exists()]
    if ausentes:
        raise SystemExit("Binarios ausentes: " + ", ".join(ausentes))
    resultados = {}
    for escenario, iteraciones in ESCENARIOS.items():
        resultados[escenario] = {}
        for modo in BINARIOS:
            print(f"Midiendo {escenario}/{modo}", flush=True)
            resultados[escenario][modo] = medir_escenario(modo, escenario, iteraciones)
    cache = {}
    rss = {}
    for modo in ("automatico", "sin_jit", "sin_feature"):
        cache[modo] = medir_cache(modo)
        rss[modo] = {escenario: rss_maximo(modo, escenario) for escenario in ESCENARIOS}
    entorno = {
        "sistema": platform.platform(),
        "maquina": platform.machine(),
        "procesador": platform.processor(),
        "python": platform.python_version(),
        "rust": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "cpu": next((linea.split(":", 1)[1].strip() for linea in Path("/proc/cpuinfo").read_text().splitlines() if linea.startswith("model name")), "desconocida") if Path("/proc/cpuinfo").exists() else "desconocida",
    }
    documento = {"entorno": entorno, "escenarios": resultados, "cache": cache, "rss_maximo_kib": rss}
    (SALIDA / "resultados.json").write_text(json.dumps(documento, ensure_ascii=False, indent=2) + "\n")
    with (SALIDA / "resultados.csv").open("w", newline="") as archivo:
        escritor = csv.writer(archivo)
        escritor.writerow(["escenario", "modo", "muestras", "mediana_ms", "desviacion_ms", "minimo_ms", "maximo_ms"])
        for escenario, modos in resultados.items():
            for modo, valores in modos.items():
                escritor.writerow([escenario, modo, valores["muestras"], valores["mediana_ms"], valores["desviacion_ms"], valores["minimo_ms"], valores["maximo_ms"]])
    lineas = ["# Informe de rendimiento", "", "Mediciones de procesos release; el tiempo no incluye compilación de Rust. `aceleracion = tiempo_sin_jit / tiempo_automatico`.", "", "## Escenarios", "", "| Escenario | Sin JIT (ms) | Automático (ms) | Aceleración | Δ automático |", "|---|---:|---:|---:|---:|"]
    for escenario in ESCENARIOS:
        sin = resultados[escenario]["sin_jit"]["mediana_ms"]
        auto = resultados[escenario]["automatico"]["mediana_ms"]
        lineas.append(f"| {escenario} | {sin:.3f} | {auto:.3f} | {sin/auto:.3f}× | {(auto/sin-1)*100:.2f} % |")
    lineas += ["", "## Histórico", "", "| Escenario | v0.0.2 (ms) | 0.0.3 original (ms) | Actual sin feature (ms) |", "|---|---:|---:|---:|"]
    for escenario in ESCENARIOS:
        lineas.append(f"| {escenario} | {resultados[escenario]['v002']['mediana_ms']:.3f} | {resultados[escenario]['base_003']['mediana_ms']:.3f} | {resultados[escenario]['sin_feature']['mediana_ms']:.3f} |")
    lineas += ["", "## Caché y memoria", "", "| Modo | Fría (ms) | Caliente (ms) | RSS corto (KiB) | RSS caliente (KiB) |", "|---|---:|---:|---:|---:|"]
    for modo in cache:
        lineas.append(f"| {modo} | {cache[modo]['fria_ms']:.3f} | {cache[modo]['caliente_mediana_ms']:.3f} | {rss[modo]['corto']} | {rss[modo]['caliente']} |")
    lineas += ["", "Los datos completos, dispersión, tamaños de muestra y entorno están en `resultados.json` y `resultados.csv`."]
    (SALIDA / "INFORME_RENDIMIENTO.md").write_text("\n".join(lineas) + "\n")

if __name__ == "__main__":
    main()
