# Especificación 001: Generación y actualización de skills

## Contexto y objetivo

Las personas que usan agentes de IA necesitan convertir documentación pública de librerías en skills reutilizables y mantenibles. El MVP permitirá crear una skill desde una URL de documentación y actualizar las skills que haya generado la CLI, preservando su procedencia y evitando la pérdida de contenido existente.

## Usuarios

- Personas desarrolladoras que desean proporcionar a un agente de IA conocimiento verificable sobre una librería.
- Mantenedores de skills que necesitan refrescarlas cuando cambia su documentación de origen.

## Historias de usuario

- Como persona desarrolladora, quiero crear una skill indicando una URL y un nombre para que un agente pueda consultar la documentación de una librería.
- Como mantenedor, quiero actualizar todas las skills creadas por la CLI sin indicar argumentos para mantenerlas vigentes.
- Como mantenedor, quiero actualizar solo una o varias skills y cambiar su nombre, URL o alcance para controlar la actualización.
- Como persona usuaria, quiero que la CLI no sobrescriba una skill existente ni deje una actualización incompleta para no perder trabajo.

## Requisitos funcionales

### RF-1. Creación de una skill

- Cuando la persona usuaria solicite crear una skill, la CLI deberá requerir una URL de origen y un nombre de skill.
- Si la URL de origen no usa HTTP o HTTPS, no es válida, no es accesible o responde con una redirección, la CLI deberá informar el error y no deberá crear una skill parcial.
- Si el nombre no es un slug de hasta 64 caracteres formado solo por letras minúsculas ASCII, números y guiones, y no empieza ni termina con un guion, la CLI deberá rechazar la creación y no deberá crear una skill parcial.
- Cuando se cree correctamente una skill, la CLI deberá ubicarla bajo `.agents/skills/<nombre-de-skill>` relativo al directorio desde el que se ejecutó.

### RF-2. Protección frente a sobrescritura

- Si ya existe una skill con el nombre solicitado, la CLI deberá rechazar la creación y no deberá modificar su contenido.
- Si una actualización no puede completarse, la CLI deberá conservar sin cambios la versión previa de la skill afectada.

### RF-3. Descubrimiento de documentación relacionada

- Cuando la persona usuaria cree o actualice una skill, la CLI deberá permitir elegir mediante parámetros uno de estos alcances: mismo sitio, prefijo de ruta, directorio padre o navegación de documentación.
- Si la persona usuaria no indica un alcance, la CLI deberá usar el alcance de mismo sitio.
- Cuando se seleccione el alcance de mismo sitio, la CLI deberá permitir elegir como límite de sitio: mismo host exacto, mismo dominio base o mismo origen.
- Si la persona usuaria no indica un límite para el alcance de mismo sitio, la CLI deberá usar el mismo host exacto, incluido el subdominio y el puerto, y deberá permitir cambios entre HTTP y HTTPS.
- Cuando se seleccione el límite de mismo dominio base, la CLI deberá incluir un subdominio distinto únicamente si la persona usuaria lo autorizó explícitamente y existe un enlace hacia él desde una página ya incluida.
- Cuando se seleccione el límite de mismo origen, la CLI deberá incluir únicamente páginas con el mismo esquema, host y puerto que la URL de origen.
- Cuando se seleccione el alcance de prefijo de ruta, la CLI deberá incluir únicamente URLs con el mismo prefijo de ruta que la URL de origen hasta su último segmento.
- Cuando se seleccione el alcance de directorio padre, la CLI deberá incluir únicamente URLs bajo el directorio padre de la URL de origen.
- Cuando se seleccione el alcance de navegación de documentación, la CLI deberá incluir únicamente enlaces presentes en elementos HTML de navegación de la página inicial.
- Cuando la persona usuaria no indique un modo de recorrido, la CLI deberá seguir enlaces repetidamente hasta no encontrar páginas nuevas dentro del alcance.
- Cuando la persona usuaria seleccione el recorrido de un nivel, la CLI deberá incluir únicamente enlaces directos de la página inicial.
- Cuando la persona usuaria seleccione el recorrido con límite configurable y proporcione un máximo, la CLI deberá aceptar únicamente un número entero positivo de páginas; si no proporciona un máximo, deberá usar 100 y detener el descubrimiento al alcanzar ese límite.
- Cuando se alcance el máximo de páginas, la CLI deberá generar la skill con las páginas extraídas hasta ese momento, siempre que no se haya producido otro fallo.
- La CLI deberá considerar iguales dos URLs que difieran solo por fragmento o parámetros de consulta y no deberá extraerlas más de una vez.
- Cuando la URL inicial sea extraíble, la CLI deberá incluirla siempre en la skill.
- Cuando la persona usuaria cree o actualice una skill, la CLI deberá permitir configurar si exige la existencia de un archivo `robots.txt` válido durante el descubrimiento.
- Si la persona usuaria no configura la exigencia de `robots.txt`, la CLI no deberá requerir que el sitio publique ese archivo para descubrir documentación relacionada.
- Cuando la persona usuaria configure la exigencia de `robots.txt`, la CLI deberá tratar la operación para esa skill como fallida y no deberá guardar una creación ni actualización parcial si no encuentra o no puede validar un archivo `robots.txt` aplicable.
- Cuando el sitio publique un archivo `robots.txt` válido, la CLI deberá respetar sus reglas aplicables independientemente de que la persona usuaria haya configurado su exigencia.
- Si la URL inicial responde con una redirección, devuelve contenido HTML que no puede extraerse, está prohibida por un archivo `robots.txt` válido o por las condiciones de acceso aplicables, la CLI deberá tratar la operación para esa skill como fallida y no deberá guardar una creación ni actualización parcial.
- Si no se encuentra ninguna página de documentación válida para incluir, la CLI deberá tratar la operación para esa skill como fallida y no deberá guardar una creación ni actualización parcial.

### RF-4. Contenido y procedencia de la skill

- Cuando la extracción finalice correctamente, la CLI deberá generar una skill a partir de contenido extraído de las páginas incluidas.
- La skill deberá preservar las URL de origen y atribuir el contenido a sus fuentes.
- La skill deberá conservar metadatos validables que identifiquen que fue generada por la CLI y permitan recuperar la URL de origen, el alcance, el límite de sitio, el modo de recorrido, el máximo de páginas, la exigencia de `robots.txt` y el formato de contenido para actualizaciones posteriores.
- Cuando la persona usuaria no indique un formato de contenido, la CLI deberá generar una guía con referencias que incluya el objetivo de la librería, instrucciones para usar la skill, contenido extraído organizado y la URL de cada fuente.
- Cuando la persona usuaria seleccione el formato de contenido organizado, la CLI deberá generar únicamente el contenido extraído organizado por página o tema y las URL de sus fuentes.
- La CLI no deberá presentar como documentación contenido que no se haya extraído de las fuentes incluidas.
- La CLI deberá derivar exclusivamente del contenido extraído el objetivo de la librería y las instrucciones para usar la skill.

### RF-5. Actualización sin argumentos

- Cuando la persona usuaria solicite actualizar sin argumentos, la CLI deberá actualizar todas y solo las skills generadas previamente por la CLI.
- Si no existen skills generadas previamente por la CLI, la CLI deberá informar que no hay skills que actualizar y no deberá modificar ninguna skill.
- Cuando una skill gestionada haya sido modificada manualmente, la CLI deberá reemplazar su contenido al actualizarla correctamente.

### RF-6. Actualización dirigida y cambios de configuración

- Cuando la persona usuaria indique una o varias skills para actualizar, la CLI deberá actualizar únicamente las skills indicadas que hayan sido generadas previamente por la CLI.
- Cuando la persona usuaria indique un nuevo nombre, URL, alcance, límite de sitio, modo de recorrido, máximo de páginas, exigencia de `robots.txt` o formato de contenido para una única skill seleccionada, la CLI deberá usar los valores indicados en esa actualización y conservarlos para las siguientes.
- Si el nuevo nombre no cumple las reglas de slug y longitud de RF-1, la CLI deberá rechazar la actualización y no deberá modificar la skill.
- Si el nuevo nombre coincide exactamente con el nombre actual de la skill, la CLI deberá actualizarla normalmente.
- Cuando la persona usuaria seleccione varias skills para actualizar y proporcione cambios de nombre, URL, alcance, límite de sitio, modo de recorrido, máximo de páginas, exigencia de `robots.txt` o formato de contenido, la CLI deberá rechazar toda la operación y no deberá modificar ninguna skill.
- Cuando la persona usuaria seleccione varias skills para actualizar sin cambios de configuración, la CLI deberá actualizar cada skill válida con su configuración existente.
- Si una skill indicada no existe o no fue generada por la CLI, la CLI deberá reportar un fallo para esa skill y continuar con las restantes válidas.
- Si los metadatos de una skill gestionada faltan, son inválidos o no coinciden con su contenido, la CLI deberá solicitar confirmación para reconstruirlos y actualizar la skill.
- Si la persona usuaria rechaza o no responde a la confirmación de reconstrucción de metadatos, la CLI no deberá modificar esa skill.
- Si el nuevo nombre entra en conflicto con una skill existente, la CLI deberá rechazar la actualización y no deberá modificar ninguna de las skills implicadas.

### RF-7. Resultado de las operaciones

- Cuando una operación finalice, la CLI deberá comunicar las skills creadas o actualizadas y las que hayan fallado, junto con la causa de cada fallo.
- Si una actualización de varias skills contiene fallos, la CLI deberá continuar con las restantes, conservar la versión previa de cada skill fallida e indicar el resultado individual de cada skill.
- Si una operación contiene al menos una skill fallida, la CLI deberá finalizar con un código de salida distinto de cero.

### RF-8. URLs de documentación no encontradas

- Cuando la URL base responda con cualquier error HTTP o de red, la CLI deberá tratar la operación para esa skill como fallida y no deberá guardar una creación ni actualización parcial.
- Cuando una URL de documentación relacionada responda con HTTP 404, la CLI deberá continuar la generación de la skill.
- Cuando una URL de documentación relacionada responda con HTTP 404, la CLI deberá incluir en la skill una mención de que no hay documentación disponible para esa URL y recomendar buscar información en internet o en el código fuente correspondiente.
- Cuando una URL de documentación relacionada responda con un error distinto de HTTP 404, la CLI deberá tratar la operación para esa skill como fallida y no deberá guardar una creación ni actualización parcial.

### RF-9. Omisión de recursos no HTML

- Cuando la CLI obtenga una respuesta correcta para una URL de documentación cuyo tipo de contenido no sea HTML, deberá omitir esa URL y su contenido de la skill, y deberá continuar el descubrimiento y la generación con las demás URLs disponibles.

### RF-10. Descubrimiento previo mediante sitemap

- Cuando la persona usuaria cree o actualice una skill, antes de recorrer enlaces HTML la CLI deberá buscar un sitemap del sitio de documentación mediante una lista determinista de ubicaciones convencionales que incluya `/sitemap.xml`, `/sitemap_index.xml`, `/sitemap-index.xml` y `/sitemap.php`.
- Cuando una ubicación convencional responda con un sitemap XML válido, la CLI deberá obtener sus URLs canónicas de documentación antes de iniciar el recorrido de enlaces HTML.
- Cuando un sitemap XML válido sea un índice de sitemaps, la CLI deberá procesar sus sitemaps referenciados de forma determinista antes de iniciar el recorrido de enlaces HTML.
- Cuando un sitemap proporcione URLs canónicas dentro del alcance, el límite de sitio y el modo de recorrido configurados, la CLI deberá usarlas como candidatas de descubrimiento y no deberá extraer una misma URL más de una vez.
- Cuando un sitemap proporcione una URL fuera del alcance, del límite de sitio o prohibida por un archivo `robots.txt` válido o por las condiciones de acceso aplicables, la CLI no deberá extraer esa URL.
- Cuando ninguna ubicación convencional proporcione un sitemap XML válido, cuando un sitemap no pueda obtenerse, sea inválido, redirija o responda con un error HTTP, la CLI deberá continuar con el descubrimiento normal mediante enlaces HTML y no deberá fallar la operación solo por ese motivo.
- Cuando el sitemap no proporcione páginas de documentación válidas, la CLI deberá continuar con el descubrimiento normal mediante enlaces HTML.
- Cuando el sitio publique un sitemap XML válido y el recorrido posterior por enlaces HTML encuentre páginas adicionales dentro del alcance, la CLI deberá incluir también esas páginas conforme al modo de recorrido configurado.

## Requisitos no funcionales

- La CLI deberá respetar las reglas aplicables de todo archivo `robots.txt` válido que encuentre, los términos de uso aplicables y los límites de acceso de cada sitio; salvo configuración expresa, la ausencia de `robots.txt` no impedirá el descubrimiento.
- La CLI deberá identificar sus solicitudes con un `User-Agent` configurable, aplicar tiempos de espera, reintentos acotados y limitación de concurrencia.
- La CLI deberá tratar HTML, URLs y contenido remoto como no confiables y no deberá permitir que determinen rutas de escritura fuera del directorio de salida definido.
- Para iguales fuentes y configuración, la generación deberá producir resultados deterministas.
- La CLI deberá emitir mensajes de uso y de error en inglés.
- Si no puede crear, reemplazar o renombrar una skill por permisos, espacio insuficiente, una ruta bloqueada o una interrupción, la CLI deberá fallar esa skill sin dejar cambios.
- Cuando dos ejecuciones intenten modificar la misma skill simultáneamente, la CLI deberá permitir una sola operación y rechazar la otra sin cambios.

### Dependencias justificadas

- Se justifican dependencias para solicitudes HTTP(S) con control de redirecciones, cabeceras, tiempos de espera y reintentos, necesarias para obtener documentación pública de forma controlada. Cubre RF-1 y RF-3.
- Se justifican dependencias para analizar y normalizar URLs, resolver enlaces relativos y aplicar los límites de descubrimiento. Cubre RF-1 y RF-3.
- Se justifican dependencias para identificar dominios registrables mediante una lista de sufijos públicos al aplicar el límite de mismo dominio base. Cubre RF-3.
- Se justifican dependencias para analizar HTML de forma estructurada y extraer enlaces, navegación y contenido documental sin interpretar HTML como texto no estructurado. Cubre RF-3 y RF-4.
- Se justifican dependencias para analizar XML de sitemap de forma estructurada, distinguir índices de sitemaps y obtener URLs canónicas sin interpretar XML como texto no estructurado. Cubre RF-10.
- Se justifican dependencias para serializar y validar los metadatos JSON que identifican una skill gestionada y conservan su configuración de actualización. Cubre RF-4, RF-5 y RF-6.
- Se justifican dependencias para calcular y verificar una huella criptográfica del contenido gestionado y detectar modificaciones antes de reconstruir metadatos o actualizar la skill. Cubre RF-4 y RF-6.
- Se justifican dependencias para analizar y validar los argumentos de los comandos de la CLI de forma consistente con su contrato público. Cubre RF-1, RF-3, RF-5 y RF-6.

## Casos límite

- La URL base redirige, no responde, responde con cualquier error HTTP, está prohibida por las condiciones de acceso aplicables o devuelve contenido HTML no apto para extraer documentación: la operación falla sin cambios parciales.
- Una URL de documentación relacionada responde con HTTP 404: la generación continúa e incluye una mención de ausencia de documentación para esa URL con la recomendación de buscar en internet o en el código fuente correspondiente.
- Una URL de documentación relacionada redirige, no responde, responde con un error distinto de HTTP 404, está prohibida por las condiciones de acceso aplicables o devuelve contenido HTML no apto para extraer documentación: la operación falla sin cambios parciales.
- La URL inicial o una URL de documentación relacionada responde correctamente con contenido no HTML, como una imagen, un PDF, una hoja de estilo, un script o un archivo descargable: la CLI omite esa URL y continúa con las demás URLs disponibles; si no queda ninguna página de documentación válida, la operación falla sin cambios parciales.
- El sitio no publica `robots.txt`, publica un archivo no válido o este no puede obtenerse: el descubrimiento continúa si no se exige su existencia y falla sin cambios parciales si se exige.
- Un archivo `robots.txt` válido prohíbe acceder a la URL inicial o a una página relacionada, tanto si se exige su existencia como si no.
- Una URL relacionada sale del alcance seleccionado, apunta a un subdominio no autorizado, vuelve a una página ya visitada o difiere de una URL ya visitada solo por fragmento o parámetros de consulta.
- El recorrido con límite configurable recibe cero, un valor negativo o un valor no numérico, o alcanza su máximo de páginas.
- El alcance elegido no encuentra páginas de documentación válidas, aunque la URL inicial sea accesible.
- El nombre solicitado incluye caracteres no permitidos, empieza o termina con un guion, o supera 64 caracteres.
- Ya existe una skill con el nombre de creación o con el nuevo nombre solicitado durante una actualización.
- La persona usuaria selecciona una skill inexistente o una que no fue generada por la CLI.
- No hay skills generadas por la CLI cuando se solicita una actualización sin argumentos.
- Una actualización cambia el nombre de una skill y falla antes de completarse.
- Se solicitan cambios de nombre, URL, alcance, límite de sitio, modo de recorrido, máximo de páginas, exigencia de `robots.txt` o formato al actualizar varias skills.
- Una skill tiene metadatos faltantes, inválidos o que no coinciden con su contenido, y la persona usuaria acepta o rechaza reconstruirlos.
- Una skill gestionada fue modificada manualmente antes de actualizarla.
- La escritura falla o dos ejecuciones intentan modificar la misma skill simultáneamente.
- El sitio no publica ninguna de las ubicaciones convencionales de sitemap, todas responden con 404, devuelven contenido no XML, son inválidas, redirigen o responden con otro error: la CLI continúa con el descubrimiento mediante enlaces HTML.
- Un sitemap válido es un índice que referencia uno o varios sitemaps, contiene URLs duplicadas o contiene URLs que solo difieren por fragmento o parámetros de consulta: la CLI procesa los sitemaps y extrae cada URL canónica admisible una sola vez.
- Un sitemap válido contiene URLs fuera del alcance o límite de sitio, URLs prohibidas por `robots.txt` o por las condiciones de acceso aplicables, o no contiene ninguna página documental válida: esas URLs se omiten y la CLI continúa con el recorrido normal de enlaces HTML.

## Fuera de alcance

- Extraer contenido que requiera autenticación, eludir controles de acceso o ignorar límites de peticiones.
- Descubrir o extraer sitios externos al origen, salvo subdominios explícitamente autorizados y enlazados desde una página ya incluida cuando se use el límite de mismo dominio base.
- Editar manualmente una skill existente desde la CLI.
- Persistir telemetría, credenciales o datos distintos de la información necesaria para identificar y actualizar las skills generadas.
- Seleccionar interactivamente páginas individuales descubiertas.
- Soporte para proveedores concretos de IA, integraciones externas o publicación remota de skills.

## Criterios de finalización

- La CLI permite crear una skill desde una URL y un nombre, en la ubicación acordada, sin sobrescribir una existente.
- La skill generada contiene documentación extraída, sus URL de origen y atribución.
- La CLI permite actualizar todas las skills generadas o un subconjunto de ellas, incluyendo cambios de nombre, URL y alcance.
- La CLI permite elegir el alcance y el formato de contenido acordados, y aplica sus valores predeterminados cuando no se indican.
- Las operaciones fallidas no alteran una skill existente ni dejan resultados parciales.
- Una actualización de varias skills continúa tras un fallo y comunica el resultado individual de cada una.
- La CLI identifica las skills gestionadas mediante metadatos validables y solicita confirmación antes de reconstruirlos.
- La CLI rechaza redirecciones, URLs fuera del alcance y accesos prohibidos sin generar o actualizar parcialmente la skill afectada.
- La CLI consulta las ubicaciones convencionales de sitemap antes del recorrido de enlaces HTML, usa las URLs canónicas admisibles que encuentre y continúa normalmente cuando no hay un sitemap utilizable.
- Los requisitos funcionales y los casos límite cuentan con pruebas deterministas actualizadas.

## Dudas abiertas

No hay dudas abiertas.
