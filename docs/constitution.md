# Constitucion

1. Usar Rust estable, Cargo y la biblioteca estandar salvo que una spec justifique una dependencia.
2. Cada cambio de comportamiento debe derivar de una spec aprobada; el código no debe excederla.
3. Los handlers de la CLI solo orquestan; red, parseo, modelos y salida son capas independientes.
4. Agregar o actualizar tests deterministas para cada comportamiento publico y corrección de errores.
5. No persistir datos, telemetría ni credenciales salvo que una spec aprobada lo exija.
6. Escribir código, tests, documentación técnica, textos de CLI y errores en inglés; la constitucion y las specs en español.
