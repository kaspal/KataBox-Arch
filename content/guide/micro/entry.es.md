Un **microservicio** es un servicio pequeño, desplegado por separado, que se encarga de una parte del negocio, como los pagos o los perfiles de usuario, junto con los datos de esa parte. Los demás servicios llegan a él por la red, a través de su API o de los eventos que publica.

La clave es la independencia. Cada servicio se puede publicar, escalar y corregir sin tocar los demás, así que los equipos avanzan más rápido y una función con mucha carga puede crecer por su cuenta. El precio es que lo que antes eran llamadas a funciones ahora cruza la red: tarda más, puede fallar y necesitas trazas para seguir una petición a través de los servicios.

En el simulador, una tarjeta de Microservicio se comporta como un Servidor de aplicaciones: un grupo de instancias, cada una atendiendo un número fijo de peticiones a la vez, y cada petición conserva su hueco hasta que responden las llamadas que hace. Puede ocupar cualquier paso de cómputo de la ruta de una petición.
