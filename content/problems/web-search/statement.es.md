# Buscador web

## El producto

Tienes un buscador web. La gente escribe una consulta y espera una página de resultados al instante. Detrás de la caja de búsqueda, un rastreador va encontrando páginas y las envía para indexarlas. Las consultas no se reparten por igual: un grupo pequeño de búsquedas populares es una gran parte del total, y cuando pasa algo en las noticias, muchísima gente busca lo mismo a la vez. Que los resultados tengan unos minutos no importa. Una caja de búsqueda que devuelve errores, sí.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 8000 rps |
| Latencia p95 | 200 ms o menos |
| Disponibilidad | 99,9 % o más |
| Tasa de errores | 0,1 % o menos |
| Presupuesto | 3200 $ al mes |

## El tráfico

Las peticiones llegan a tus App Servers, a través de un edge si quieres uno.

- **Buscar en la web** (95 %). Un App Server ejecuta la consulta en un índice de búsqueda, a través de una caché si añades una. Hay 1 000 000 de consultas distintas, y unas pocas populares se llevan una gran parte (Zipf 1,0).
- **Indexar una página rastreada** (5 %). Un App Server escribe la página en el índice de búsqueda. Las escrituras van al primario del índice. Hay 10 000 000 de páginas distintas.

En este problema no hay servicios externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Cae un nodo de búsqueda** y **Todo el mundo busca lo mismo**. Tu diseño tiene que superar las tres.
