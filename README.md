# GabeGTA6

Uma camada de compatibilidade que roda o GTA VI do Xbox Series no Windows,
**voltada para placas NVIDIA e PCs fracos**. O alvo é o PC do projeto:

| | |
|---|---|
| Placa | GeForce GTX 1060 6 GB (sem mesh shaders, sem ray tracing) |
| Processador | Ryzen 7 2700 (8 núcleos) |
| Memória | 32 GB |
| Sistema | Windows 10 Pro |

**Este repositório não tem nenhum arquivo, código ou asset do jogo.** Você
precisa da sua própria cópia, tirada do seu Xbox. O projeto não tem ligação com
a desenvolvedora nem com a distribuidora do jogo.

## O que o GabeGTA6 faz de diferente

O projeto base foi feito para uma Radeon RX 7900 XTX de 24 GB com 64 GB de RAM.
Aqui ele foi adaptado para uma NVIDIA pequena:

- **Mesh shaders sem mesh shaders.** A GTX 1060 não tem; os programas de malha
  do jogo viram programas de computação e um vertex shader desenha os
  triângulos que eles escrevem.
- **Windows 10.** O DirectX 12 Agility SDK vai ao lado do executável
  (`scripts\get-agility-sdk.ps1`) para a placa ter shader model 6.6+.
- **Shaders traduzidos para a placa.** Ondas de 32 da NVIDIA, e o cache feito de
  antemão é o mesmo que o jogo procura.
- **Feito para 6 GB de vídeo e 32 GB de RAM.** 720p com o upscaler do próprio
  jogo, escala de render ajustável, texturas grandes começando num nível menor,
  mais dados do jogo guardados na placa, limites de memória para 32 GB.
- **Sem ray tracing**, que não funciona numa GTX: os ~1.000 programas dele nem
  são compilados.
- **Nada no C:.** Caches e saves ficam ao lado da pasta do projeto.

## Como usar

O passo a passo completo (compilar, arquivo de paginação, cache da NVIDIA,
rodar) está em **[docs/gtx1060.md](docs/gtx1060.md)**. Em resumo:

```powershell
git clone https://github.com/GabrieelDuart/GabeGTA6
cd GabeGTA6
cargo build --locked --profile diag -p victory --features diagnostics
scripts\get-agility-sdk.ps1
copy victory.example.toml victory.toml        # e ajuste os caminhos
target\diag\victory.exe gpu                   # confere a placa
scripts\play-1060.ps1 -Shaders <os arquivos de shaders>   # uma vez
scripts\play-1060.ps1 -Exe <executável da sua cópia>
```

Se estiver pesado: `-Scale 0.5` (resolução interna menor) e `-MipDrop 3`
(texturas mais leves). Se algo der errado: `-Safe`.

Os nomes internos (`victory.exe`, as variáveis `VICTORY_...`, as crates) ficaram
como estão: mudar quebraria a compatibilidade com o código e as correções que
vêm dos outros projetos.

## Estado

Compila e passa nos testes do tradutor. **Ainda não foi testado com o jogo numa
GTX 1060.** O que outros mediram em placas parecidas está em
[docs/gtx1060.md](docs/gtx1060.md).

## Agradecimentos

O GabeGTA6 foi montado em cima do trabalho destes projetos e pessoas:

- **[VIctory](https://github.com/Marvisak/VIctory)**, de Marvisak e
  colaboradores: o loader, os serviços do runtime, o Direct3D 12 e o tradutor
  de shaders.
- **[VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)**, de
  avatarmanz: o projeto base deste repositório. O README original dele está em
  [docs/projeto-base.md](docs/projeto-base.md), com os créditos de quem veio
  antes (fxceless, DexNom, Belobug, FQY55kai, myokex, Geckomacabre e outros).
- **[kelvaroq](https://github.com/fablinoo/kelvaroq)**, de fablinoo: texturas
  num pedaço de cada quadro, espera curta por pipelines.
- **[cool_fox](https://github.com/DoubleGhosty/cool_fox)** (DoubleGhosty) e
  **[cgbj-reupload](https://github.com/cputroll/cgbj-reupload)** (cputroll),
  com o trabalho de codec-d: pular os programas de ray tracing.
- **[nvidia-3070-VI](https://github.com/grimygunnz/nvidia-3070-VI)**, de
  grimygunnz e codec-d: mais cópias de buffers na placa.
- **[victory-gtx1660](https://github.com/Impre-visible/victory-gtx1660)**, de
  Impre-visible: texturas começando num nível menor, medidas numa placa de 6 GB.
- **[VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)**, de
  trustno1-coder: o limite de descritores da NVIDIA e o fim rápido dos raios.

O histórico do git foi mantido: cada commit trazido de outro projeto diz de
onde veio e quem fez.

## Licença

GPL-3.0-or-later, a mesma do VIctory. Veja [LICENSE](LICENSE). Por ser um
trabalho derivado, a licença exige manter o arquivo LICENSE, os avisos de
copyright e o crédito aos autores originais.
