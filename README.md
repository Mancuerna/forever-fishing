# Forever Fishing

Asistente de pesca de consola para Windows x64, escrito en Rust. Escucha el dispositivo de salida predeterminado mediante WASAPI loopback (no el micrófono), reconoce el inicio de una muestra de picada y pulsa Espacio para recoger. Tras soltar la tecla espera entre 0 y 1000 ms y vuelve a pulsarla para lanzar.

## Preparación

Configura **Espacio** en el juego/addon para recoger y volver a lanzar con la misma tecla. El programa no configura esos bindings. Lanza manualmente la primera vez. Activa el saqueo automático si tu configuración lo necesita.

El ejecutable integra las tres variantes OGG de [Fishing Hooked (3355)](https://www.wowhead.com/sound=3355/fishing-hooked) aportadas por el usuario. No necesita descargas ni archivos externos. Cada variante tiene su propia firma y duración; si varias coinciden en un bloque se usa la de mayor similitud.

| Variante | Archivo completo | Región activa usada |
| --- | ---: | ---: |
| ver2_1 | 3492 ms | 2229 ms |
| ver2_2 | 3163 ms | 2099 ms |
| ver2_3 | 3030 ms | 1590 ms |

La región activa excluye el silencio y la cola por debajo del 8 % del pico. No conviene usar toda la duración del archivo como ventana de pulsación. Créditos de los sonidos en `THIRD_PARTY.md`.

## Uso

Prueba primero sin enviar teclas:

```powershell
.\forever-fishing.exe run --dry-run
```

Vuelve al juego y pulsa **F8** para activar. Escucha y compara las detecciones de consola con las picadas reales. Después:

```powershell
.\forever-fishing.exe run
```

- **F8** activa/pausa; **Escape** termina. Empieza pausado.
- Al perder el foco del juego se pausa y cancela acciones pendientes. Pulsa F8 en el juego para reanudar.
- Por defecto exige que el título de la ventana activa contenga `World of Warcraft`. Puedes cambiarlo con `--window "título del cliente"`.
- `--threshold 0.96` ajusta la similitud mínima (0.5–1). Subirlo reduce sensibilidad; bajarlo aumenta falsos positivos.
- Usa el mismo dispositivo de audio de salida para WoW y Windows. Reinicia el asistente si cambias de dispositivo.

## Detección y tiempos

Compara seis ventanas espectrales FFT de 20 ms en 20 bandas, con diez alineaciones posibles del inicio de la muestra con el audio entrante, normalizadas para tolerar cambios de volumen. No espera a escuchar el sonido completo: tras los primeros 120 ms elige un retardo aleatorio dentro de la duración restante estimada, con margen para latencia. No puede seleccionar instantes anteriores a la detección ni garantizar que una muestra diferente dure lo mismo. Las pulsaciones duran 20 ms; el intervalo posterior incluye unos pocos milisegundos de procesamiento. La aleatoriedad no garantiza detección perfecta ni cambios en cómo el juego interpreta las teclas.

Descarta audio atrasado y acciones vencidas, e ignora detecciones mientras recoge/relanza y durante dos segundos tras relanzar. Una picada perdida no provoca relanzamientos automáticos: pausa, lanza manualmente y reanuda. La captura mezcla todo el audio del PC: desactiva música/notificaciones para mejorar fiabilidad. La identificación por título de ventana no identifica un proceso de forma inequívoca.

Las pruebas reproducen las tres muestras con veinte desfases temporales y volumen al 20 %, y comprueban rechazo de silencio, otro tono y ruido blanco. La peor similitud observada es 0.9961; la detección ocurre antes de 180 ms desde el inicio de la región activa. **Pendiente de prueba dentro de WoW Forever**, con su mezcla de audio real.

## Referencias personalizadas

Puedes sustituir las muestras integradas por uno o varios archivos WAV u OGG Vorbis:

```powershell
.\forever-fishing.exe run picada1.ogg picada2.ogg picada3.wav --dry-run
.\forever-fishing.exe record captura.wav --seconds 5
```

Para crear una referencia, recorta una sola picada completa de la grabación, sin lanzamiento, música ni otros efectos. Se admiten archivos de hasta 30 segundos con región activa de 250 ms a 10 s. La conversión a mono/16 kHz es interna; no necesita FFmpeg.

## Desarrollo y releases

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
```

Rust 1.99.1, Windows. Ejecutable en `target/release/forever-fishing.exe`.

Se utiliza **release-please**, siguiendo la configuración de `Mancuerna/ndpiviewer-rs`: Conventional Commits (`feat:`, `fix:`...), PR de versión/changelog, release en borrador y publicación solo tras compilar y probar. El workflow adjunta un ZIP Windows x64 con ejecutable, README y licencia, además de SHA-256. Activa en GitHub Settings → Actions → General la opción que permite a GitHub Actions crear pull requests. No requiere un PAT. `workflow_dispatch` permite reintentar una versión cuyo tag ya exista y coincida con Cargo.toml.

Referencias técnicas: [WASAPI Rust](https://github.com/HEnquist/wasapi-rs), [release-please de referencia](https://github.com/Mancuerna/ndpiviewer-rs/blob/main/.github/workflows/release.yml).
