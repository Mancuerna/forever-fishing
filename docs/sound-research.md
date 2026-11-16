# Inventario de sonidos de picada (2026-11-16)

## Resultado

Se han identificado cuatro referencias de picada estándar. Esto no prueba que Forever reproduzca las cuatro ni descarta archivos todavía sin nombre, sonidos personalizados o cambios de cliente.

| Archivo | FileDataID | Procedencia | Estado |
| --- | ---: | --- | --- |
| FishingBobber_ver2_1.ogg | 569285 | Wowhead, Fishing Hooked 3355 (Live) | Integrado |
| FishingBobber_ver2_2.ogg | 568970 | Wowhead, Fishing Hooked 3355 (Live) | Integrado |
| FishingBobber_ver2_3.ogg | 569044 | Wowhead, Fishing Hooked 3355 (Live) | Integrado |
| FishBite.ogg | 569816 | Wowhead Classic CDN, ruta histórica Tradeskills | Integrado |

Fuentes consultadas:

- [Wowhead Live: Fishing Hooked](https://www.wowhead.com/sound=3355/fishing-hooked): lista exactamente las tres variantes ver2.
- [Wowhead Classic: Fishing Hooked](https://www.wowhead.com/classic/sound=3355/fishing-hooked).
- [FishBite original del CDN Classic](https://wow.zamimg.com/sound-ids/classic/enus/216/569816/FishBite.ogg): OGG descargado y decodificado correctamente; 2171 ms completos, 1608 ms de región activa.
- [Inventario wowdev](https://github.com/wowdev/wow-listfile/blob/deeca4ac550d28d49f38be5248367a0b2eceb840/parts/sound.csv): confirma IDs y rutas. Búsqueda por fishing, fishbite, bobber, fishcast y fishreel.
- [Inventario histórico fondlez](https://github.com/fondlez/wow-sounds): Vanilla incluye FishBite.wav; MoP incluye FishBite.ogg y las tres variantes ver2. Una extensión WAV histórica no implica una quinta variante acústica.
- [TheWoWDB, icono de pesca](https://thewowdb.com/icons/trade_fishing/): enlaza FishCast, FishBite y FishReelIn con sus IDs. Su descarga de audio devolvió un desafío Cloudflare; se usó el CDN de Wowhead.

No se añadieron FishCast (569807) ni FishReelIn (569808): lanzamiento y recogida no son señales de picada. Tampoco los GO_Goblin_WaterBobber de la categoría DOODAD: su nombre no demuestra que sean avisos de picada. Los archivos `.meta` no son audio adicional.

## Por qué pueden perderse picadas aunque esté la muestra

El detector compara la salida completa del PC con referencias limpias. Música, ambiente, voces, reverberación o cambios de tono pueden alterar la firma. No se dispone aún de una grabación de una picada perdida de este usuario para atribuir el fallo a una causa concreta.

Experimento reproducible: `cargo run --example noise_probe`. Señal de referencia al 20 % de volumen, ruido blanco determinista de amplitud total 0.03. Mejores similitudes en los primeros 600 ms:

| Muestra | Similitud |
| --- | ---: |
| FishBite | 0.9589 |
| ver2_1 | 0.9447 |
| ver2_2 | 0.9180 |
| ver2_3 | 0.9440 |

Todas quedan por debajo de 0.96. El ruido sintético no representa necesariamente la mezcla del usuario, pero demuestra que añadir muestras no garantiza resolver las pérdidas. El umbral predeterminado se mantiene para no introducir falsos positivos sin mediciones reales.

`run --diagnostics` imprime el mejor resultado de cada segundo y el nombre de la referencia. Una coincidencia aceptada muestra `Detected`, el resultado y el retardo programado. `Reel` confirma que se ejecutó la acción. Así se distingue entre falta de coincidencia y problemas posteriores de tecla/saqueo. `--threshold` permite calibrar; conviene probar valores inferiores con `--dry-run` antes de activar pulsaciones.
