## 1

El almacén solo puede cargar cierto número de lotes por segundo. Cuando el relleno pide más del hueco para el que estaba ajustado, alguien tiene que esperar. ¿Quién debería ser, y qué decide el orden en que el Stream entrega los lotes?

## 2

Un Stream es FIFO (el primero en entrar es el primero en salir): un lote en vivo espera detrás de todos los lotes del relleno que ya están dentro, así que un relleno rápido deja los paneles desfasados. Activa "Primero lo urgente (prioridad)" en el Stream, para que un lote en vivo pase por delante de todos los lotes del relleno en cola; el relleno se acumula en el Stream, donde esperar no cuesta nada. La prioridad no ayuda cuando los lotes en vivo solos superan lo que el almacén puede cargar: dimensiona el almacén para el pico en vivo, y dale al Stream particiones suficientes y al Procesador de streams huecos suficientes para mantener ocupados todos los huecos del almacén.

## 3

Pon un AWS ALB delante de 2 Servidores de aplicaciones Go que publican en un Stream Apache Kafka con 32 Particiones y "Primero lo urgente (prioridad)" activado. Un Procesador de streams Apache Flink con 2 instancias y una Concurrencia por instancia de 16 lo consume y carga cada lote en un Almacén de datos Snowflake con 3 Instancias.
