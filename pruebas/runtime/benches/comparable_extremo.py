import argparse
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path


def escribir_programas(raiz):
    simple_qz = raiz / "simple.qz"
    simple_qz.write_text("consola.mostrar(42)\n", encoding="utf-8")
    simple_py = raiz / "simple.py"
    simple_py.write_text("print(42)\n", encoding="utf-8")

    proyecto_qz = raiz / "proyecto_qz"
    proyecto_qz.mkdir()
    (proyecto_qz / "quetzal.json").write_text(
        json.dumps(
            {
                "version": "0.1.0",
                "aplicacion": "benchmark_modulos",
                "entrada": "principal.qz",
                "tipo": "aplicacion",
                "quetzal": "0.0.2",
                "dependencias": {},
                "permisos": {},
            }
        ),
        encoding="utf-8",
    )
    (proyecto_qz / "principal.qz").write_text(
        'importar { sumar } desde "./aplicacion/calculadora.qz"\n'
        "consola.mostrar(sumar(15, 27))\n",
        encoding="utf-8",
    )
    (proyecto_qz / "aplicacion").mkdir()
    (proyecto_qz / "aplicacion" / "calculadora.qz").write_text(
        "entero sumar(entero a, entero b) {\n"
        "    retornar a + b\n"
        "}\n"
        "exportar { sumar }\n",
        encoding="utf-8",
    )

    proyecto_py = raiz / "proyecto_py"
    (proyecto_py / "aplicacion").mkdir(parents=True)
    (proyecto_py / "principal.py").write_text(
        "from aplicacion.calculadora import sumar\nprint(sumar(15, 27))\n",
        encoding="utf-8",
    )
    (proyecto_py / "aplicacion" / "__init__.py").write_text("", encoding="utf-8")
    (proyecto_py / "aplicacion" / "calculadora.py").write_text(
        "def sumar(a, b):\n    return a + b\n", encoding="utf-8"
    )
    return simple_qz, simple_py, proyecto_qz, proyecto_py


def ejecutar(comando, salida_esperada):
    inicio = time.perf_counter()
    proceso = subprocess.Popen(
        [str(parte) for parte in comando],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    rss = None
    if hasattr(os, "wait4"):
        _, estado, uso = os.wait4(proceso.pid, 0)
        retorno = os.waitstatus_to_exitcode(estado)
        rss = uso.ru_maxrss
        if sys.platform != "darwin":
            rss *= 1024
    else:
        retorno = proceso.wait()
    proceso.returncode = retorno
    duracion = time.perf_counter() - inicio
    salida = proceso.stdout.read()
    errores = proceso.stderr.read()
    if retorno != 0 or salida != salida_esperada:
        raise RuntimeError(
            f"falló {comando}: retorno={retorno}, salida={salida!r}, errores={errores!r}"
        )
    return duracion, rss


def medir(nombre, comando, repeticiones, salida_esperada="42\n", preparar=None):
    muestras = []
    memorias = []
    for _ in range(repeticiones):
        if preparar is not None:
            preparar()
        duracion, rss = ejecutar(comando, salida_esperada)
        muestras.append(duracion)
        if rss is not None:
            memorias.append(rss)
    resultado = {
        "nombre": nombre,
        "comando": [str(parte) for parte in comando],
        "mediana_segundos": statistics.median(muestras),
        "minimo_segundos": min(muestras),
        "muestras": muestras,
        "rss_maximo_bytes": max(memorias) if memorias else None,
    }
    if not memorias:
        resultado["rss_limitacion"] = "os.wait4 no está disponible en esta plataforma"
    return resultado


def borrar_cache_qz(proyecto):
    shutil.rmtree(proyecto / ".quetzal", ignore_errors=True)


def borrar_cache_py(proyecto):
    shutil.rmtree(proyecto / "aplicacion" / "__pycache__", ignore_errors=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("quetzal", type=Path, help="ruta al binario quetzal release sin JIT")
    parser.add_argument("--repeticiones", type=int, default=7)
    args = parser.parse_args()
    if args.repeticiones < 1:
        parser.error("--repeticiones debe ser positivo")
    binario = args.quetzal.resolve()
    if not binario.is_file():
        parser.error(f"no existe el binario {binario}")

    with tempfile.TemporaryDirectory(prefix="quetzal_bench_extremo_") as temporal:
        raiz = Path(temporal)
        simple_qz, simple_py, proyecto_qz, proyecto_py = escribir_programas(raiz)
        resultados = []
        for modo, argumentos in [("normal", []), ("sin_especializacion", ["--sin-optimizar"])]:
            base = [binario, "ejecutar", *argumentos]
            resultados.append(
                medir(
                    f"inicio_script_{modo}",
                    [*base, simple_qz],
                    args.repeticiones,
                )
            )
            comando = [*base, proyecto_qz]
            resultados.append(
                medir(
                    f"modulos_cache_fria_{modo}",
                    comando,
                    args.repeticiones,
                    preparar=lambda: borrar_cache_qz(proyecto_qz),
                )
            )
            ejecutar(comando, "42\n")
            resultados.append(
                medir(f"modulos_cache_caliente_{modo}", comando, args.repeticiones)
            )

        resultados.append(
            medir("cpython_inicio_script", [sys.executable, simple_py], args.repeticiones)
        )
        comando_py = [sys.executable, proyecto_py / "principal.py"]
        resultados.append(
            medir(
                "cpython_modulos_cache_fria",
                comando_py,
                args.repeticiones,
                preparar=lambda: borrar_cache_py(proyecto_py),
            )
        )
        ejecutar(comando_py, "42\n")
        resultados.append(
            medir("cpython_modulos_cache_caliente", comando_py, args.repeticiones)
        )

    print(
        json.dumps(
            {
                "python": sys.version,
                "implementacion": platform.python_implementation(),
                "plataforma": platform.platform(),
                "repeticiones": args.repeticiones,
                "notas": [
                    "Cada muestra crea un proceso nuevo y comprueba que imprime exactamente 42.",
                    "La caché fría se borra antes de cada muestra dentro del directorio temporal.",
                    "RSS es el máximo del proceso hijo; no incluye otros procesos.",
                ],
                "resultados": resultados,
            },
            ensure_ascii=False,
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
