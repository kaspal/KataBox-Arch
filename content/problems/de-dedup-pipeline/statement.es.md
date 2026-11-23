# Eventos de facturación, exactamente una vez

## El producto

Llevas el pipeline de facturación de una empresa de software que cobra por uso. Cada cosa facturable que hace un cliente (una llamada a la API por encima del nivel gratuito, un gigabyte guardado, un puesto añadido) se convierte en un evento de facturación. Los servicios que los producen envían cada evento a un servicio de ingesta en Go, que lo escribe en un stream de Kafka. Un procesador de streams lee el stream, enriquece cada evento (lista de precios, moneda, cuenta del cliente) y lo escribe como una fila en el libro de facturación, una tabla de un almacén de datos. A fin de mes, las facturas se suman a partir de esa tabla.

Finanzas tiene una regla por encima de todas: **cada evento está en el libro exactamente una vez**. Una fila que falta es ingreso perdido; una fila repetida es un cliente cobrado dos veces, y una incidencia de soporte, y un reembolso. Así que el pipeline no debe perder nada ni duplicar nada, ni siquiera cuando el procesador de streams se cae y se reinicia. Las facturas no corren prisa, pero el panel de ingresos en vivo de finanzas lee el libro, así que un evento debería llegar en 30 s como mucho.

## Lo que debes cumplir

| Objetivo | Meta |
| --- | --- |
| Tráfico pico | 40 eventos/s |
| Tiempo p95 del evento al libro | 30 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Eventos perdidos | ninguno |
| Filas duplicadas en el libro | ninguna |
| Presupuesto | 4000 $ al mes |

## El tráfico

- **Registrar un evento de facturación** (100 %). El evento puede pasar por un balanceador de carga, llega a tus servidores de aplicación, entra en un Stream, lo procesa un Procesador de streams y se escribe en un Almacén de datos. Se mide hasta que está en el almacén. Cada evento tiene su propio id (10 000 000 en juego).

Cómo se comportan las tarjetas aquí:

- El procesador de streams no usa ventanas (deja **Ventana** en `none`): cada evento se enriquece y se escribe por separado. Un hueco del procesador espera a que termine su escritura, así que el procesador necesita tantos huecos (**Concurrencia por instancia** × **Instancias**) como escrituras en curso, y un Stream entrega un evento por partición a la vez, así que sus **Particiones** limitan los huecos que pueden trabajar.
- Una escritura en un nodo del Almacén de datos tarda unos 300 ms, y cada nodo acepta 8 escrituras a la vez (los valores por defecto).
- Un Procesador de streams guarda su progreso en cada **Intervalo de checkpoint**. Cuando se cae, se reinicia desde su último checkpoint y vuelve a procesar todo lo posterior, y esas filas llegan al libro por segunda vez, salvo que el almacén tenga activadas las **Escrituras idempotentes (dedup)** (una escritura con el id del evento como clave: escribir otra vez el mismo evento no cambia nada).
- Cuando los productores reintentan, vuelven a enviar los mismos eventos. El simulador no sigue los ids de los eventos, así que trata cada evento reenviado como un evento nuevo: una tormenta de reintentos es más carga, no más duplicados. Aquí los duplicados vienen solo de las repeticiones del procesador de streams.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en el pico.

Hay dos pruebas más ocultas, y solo ves sus nombres: **El procesador de streams se cae** y **Una tormenta de reintentos: 3 veces los eventos durante 30 s, y el procesador se cae en medio**. Tu diseño tiene que pasar las tres.
