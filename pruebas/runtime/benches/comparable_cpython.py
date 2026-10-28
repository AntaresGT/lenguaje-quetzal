import json
import platform
import statistics
import sys
import time
from decimal import Decimal


def sumar_hasta(limite):
    suma = 0
    for i in range(limite):
        suma += i
    return suma


def fib(n):
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


class Punto:
    def __init__(self):
        self.x = 3


def leer_repetido(punto, limite):
    suma = 0
    for _ in range(limite):
        suma += punto.x
    return suma


def trabajar_lista(datos):
    suma = 0
    for n in datos:
        suma += n
    return suma


def sumar_uno(n):
    return n + 1


def llamar_repetido(limite):
    suma = 0
    for _ in range(limite):
        suma = sumar_uno(suma)
    return suma


def sumar_jsn(valores, limite):
    suma = 0
    for _ in range(limite):
        suma += valores["a"] + valores["b"]
    return suma


def trabajar_decimales(limite):
    suma = Decimal(0)
    for _ in range(limite):
        suma += DECIMAL_01 + DECIMAL_02
    return suma


def trabajar_unicode(limite):
    texto = "Quetzal🦜áéíóú" * 20
    suma = 0
    for i in range(limite):
        suma += len(texto[i % len(texto)])
    return suma


class PuntoCreado:
    def __init__(self):
        self.x = 3


def crear_repetido(limite):
    suma = 0
    for _ in range(limite):
        punto = PuntoCreado()
        suma += punto.x
    return suma


DATOS = list(range(1, 10000))
PUNTO = Punto()
VALORES = {"a": 20, "b": 22}
DECIMAL_01 = Decimal("0.1")
DECIMAL_02 = Decimal("0.2")


def medir(nombre, funcion, repeticiones=5):
    tiempos = []
    resultado = None
    for _ in range(repeticiones):
        inicio = time.perf_counter()
        resultado = funcion()
        tiempos.append(time.perf_counter() - inicio)
    return {
        "nombre": nombre,
        "resultado": str(resultado),
        "mediana_segundos": statistics.median(tiempos),
        "minimo_segundos": min(tiempos),
        "muestras": tiempos,
    }


def main():
    casos = [
        ("bucles_enteros", lambda: sumar_hasta(2_000_000)),
        ("llamadas_recursivas", lambda: fib(25)),
        ("llamadas_pequenas", lambda: llamar_repetido(200_000)),
        ("atributos", lambda: leer_repetido(PUNTO, 200_000)),
        ("listas", lambda: trabajar_lista(DATOS)),
        ("jsn", lambda: sumar_jsn(VALORES, 100_000)),
        ("decimales", lambda: trabajar_decimales(100_000)),
        ("unicode", lambda: trabajar_unicode(100_000)),
        ("objetos", lambda: crear_repetido(100_000)),
    ]
    salida = {
        "python": sys.version,
        "implementacion": platform.python_implementation(),
        "plataforma": platform.platform(),
        "resultados": [medir(nombre, funcion) for nombre, funcion in casos],
    }
    print(json.dumps(salida, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
