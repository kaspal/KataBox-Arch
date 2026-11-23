## 1

Los App Servers y la base de datos estaban casi sin trabajo durante la caída. ¿Qué tarjeta rechazó las peticiones, y por qué?

## 2

El limitador no podía llegar a Redis, así que no sabía si una petición estaba dentro del límite de su clave, y estaba configurado para rechazar todo lo que no pudiera contar. Para un límite general de una API, rechazar a todos es peor que no limitar a nadie durante un rato.

## 3

En el Rate Limiter, pon On failure en open. No hace falta cambiar nada más: los App Servers tienen margen para el tráfico sin límite mientras Redis no está.
