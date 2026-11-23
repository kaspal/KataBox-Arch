# Resumir un atraso de documentos

## El producto

Tu empresa guarda un gran archivo de documentos, y la página de búsqueda quiere un resumen corto junto a cada uno. Cada noche, un trabajo programado recoge los documentos que llegaron durante el día y pide a un modelo de lenguaje alojado que resuma cada uno. Nadie lee los resúmenes hasta la mañana siguiente, así que ninguna persona espera a ninguna llamada concreta. Lo que importa es que todo el atraso esté hecho antes del plazo, sin errores y sin documentos perdidos, y que la factura de tokens sea razonable.

**Tiempo escalado.** Un atraso real se resume durante la noche, a lo largo de horas. El simulador ejecuta minutos, así que el problema está reducido: el atraso llega en los primeros 30 segundos de una ejecución de dos minutos, y "antes de la mañana" pasa a ser "en 45 segundos desde que entra en la cola". Los límites por minuto del proveedor del modelo se mantienen a su tamaño real, así que lo que tienes que acertar es el atraso frente al límite.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Atraso | 15 documentos/s durante 30 s (450 documentos) |
| Plazo (p95, desde la cola hasta el resumen) | 45 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Documentos perdidos | ninguno |
| Presupuesto | 38 000 $ al mes |

El coste mensual es lo que costaría el ritmo de la ejecución si durara un mes entero, la misma regla que en todos los demás problemas. Los precios por token son los precios didácticos del registro.

## El tráfico

- **Resumir un documento** (100 %). Un Planificador publica cada documento en una Cola. Un Worker lo toma de la Cola y llama a una tarjeta de Inferencia de modelos (un modelo de chat). El tiempo que se mide va desde que el documento entra en la cola hasta que su resumen está hecho.

Un documento tiene unos 600 tokens (1200 en el p95), y su resumen unos 100 tokens (200 en el p95). La cuenta del proveedor del modelo permite 4000 peticiones y 400 000 tokens por minuto en tiempo real (el "Límite de peticiones por minuto" y el "Límite de tokens por minuto" de la tarjeta); son los límites de tu cuenta, y no puedes subirlos para este trabajo. Una llamada por encima de un límite se rechaza con un HTTP 429. Un Worker conserva su hueco durante toda la llamada al modelo.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **El atraso de esta noche**: 450 documentos encolados en los primeros 30 segundos.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El triple de atraso** y **El proveedor del modelo se vuelve lento**. Tu diseño tiene que superar las tres.
