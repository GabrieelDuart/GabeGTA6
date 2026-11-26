# GabeGTA6

Uma camada de compatibilidade que roda o GTA VI do Xbox Series no Windows,
**voltada para placas NVIDIA e PCs fracos**. O alvo é o PC do projeto:

| | |
|---|---|
| Placa | GeForce GTX 1060 6 GB (sem mesh shaders, sem ray tracing) |
| Processador | Ryzen 7 2700 (8 núcleos) |
| Memória | 32 GB |
| Sistema | Windows 10 Pro |

*In English:* GabeGTA6 is a Rust compatibility layer for the Xbox Series build
of GTA VI on Windows, tuned for small NVIDIA cards (GTX 1060 6 GB, no mesh
shaders) and 32 GB of RAM: a PE32+ loader, HLE of the console's system
libraries (kernelx, xgameruntime, d3d12_xs) and a GFX10 (RDNA2) to DXIL shader
recompiler on Direct3D 12, with a compute fallback for mesh shaders on
Pascal. No game files are included; bring your own dump.

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

## Como funciona por dentro

O jogo do Xbox é um executável PE32+ que fala com bibliotecas do console. O
GabeGTA6 faz o papel delas no Windows:

| Parte | Crate | O que faz |
|---|---|---|
| Loader | `victory-loader`, `victory-pe` | Carrega o executável PE32+ na memória, resolve imports, relocations e TLS, e aplica os patches conferidos byte a byte (resolução, escala) |
| Serviços do console (HLE) | `victory-hle` | kernelx, xgameruntime, xmem, d3d12_xs, DirectStorage, áudio XMA2, saves, controle |
| Direct3D 12 | `victory-gpu` | Command lists, descriptor heaps, pipelines, cópias de memória do console para a placa, texturas com swizzle/tiling do console |
| Tradutor de shaders | `victory-shader` | Recompila os shaders GFX10 (RDNA2) do console para DXIL, com ondas de 32 para NVIDIA e mesh shaders como compute |
| Formatos | `victory-formats` | Texturas, layouts e BCPack do console |

O que é específico da GTX 1060 está em [docs/gtx1060.md](docs/gtx1060.md).

## Status por componente

| Componente | Status | Observação |
|---|---|---|
| Compilação no Windows 10 | ✅ | Compila no PC do projeto e no GitHub Actions |
| Testes automáticos no Windows | 🟡 | 371 passam, 1 falha do código base (ver Testes no jogo) |
| Testes do tradutor de shaders | ✅ | 215 testes passando (`cargo test -p victory-shader`) |
| Clippy sem avisos | ✅ | Windows, com e sem diagnóstico |
| Agility SDK no Windows 10 | ✅ | `victory gpu` na GTX 1060: shader model 6.8, directly indexed heaps |
| Mesh shaders como compute (Pascal) | 🟡 | Feito, ainda não rodou com o jogo |
| Perfil para 6 GB / 32 GB (`play-1060.ps1`) | 🟡 | Feito, ainda não medido |
| Ray tracing | ❌ | Desligado: não funciona numa GTX |
| Rodar com o jogo | ❌ | Ainda não testado |

## O que precisa

| | |
|---|---|
| Sistema | Windows 10 ou 11, 64 bits |
| Processador | x86-64 com AVX2 (Ryzen 1000 em diante, Intel Haswell em diante) |
| Placa | DirectX 12 com shader model 6.6 (com o Agility SDK, uma GTX 1060 serve) e driver NVIDIA recente |
| Memória | 32 GB de RAM **e** um arquivo de paginação de 16 a 48 GB (o jogo usa uns 35 GB) |
| Disco | uns 10 GB para as ferramentas e a compilação, mais o cache de shaders (vários GB) |
| O jogo | a sua própria cópia, tirada do seu Xbox Series |

## Executável pronto (sem compilar)

O GitHub compila o `victory.exe` sozinho a cada mudança no `main`
(`.github/workflows/build-1060.yml`) e gera um `GabeGTA6-windows.zip` com o
executável, o Agility SDK, os scripts e a documentação. Sem nenhum arquivo do jogo.

- Na primeira vez, em **Actions** no GitHub, clique em *"I understand my
  workflows, go ahead and enable them"*.
- O .zip fica em **Actions > GabeGTA6 para Windows > a execução mais nova >
  Artifacts**. Uma tag `v0.1.0` também cria um **Release** com ele.
- Com o .zip, pule a instalação e o passo 2 abaixo: extraia no F:, e siga do
  passo 3 em diante.

## Como fazer rodar no PC

Tudo no PowerShell. Os caminhos abaixo usam o disco `F:`; troque se quiser outro.

**1. Instalar as ferramentas (uma vez)**

```powershell
# Git
winget install Git.Git
# Ferramentas de compilação da Microsoft (MSVC e Windows SDK)
winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
# Rust, instalado no F: e não no C:
[Environment]::SetEnvironmentVariable('RUSTUP_HOME', 'F:\Sistema\rust\rustup', 'User')
[Environment]::SetEnvironmentVariable('CARGO_HOME', 'F:\Sistema\rust\cargo', 'User')
$env:RUSTUP_HOME = 'F:\Sistema\rust\rustup'; $env:CARGO_HOME = 'F:\Sistema\rust\cargo'
winget install Rustlang.Rustup
```

Feche e abra o PowerShell depois de instalar. A versão certa do Rust
(`rust-toolchain.toml`) é baixada sozinha na primeira compilação.

**2. Baixar e compilar**

```powershell
cd F:\Sistema\Projetos
git clone https://github.com/GabrieelDuart/GabeGTA6
cd GabeGTA6
cargo build --locked --profile diag -p victory --features diagnostics   # uns minutos
scripts\get-agility-sdk.ps1                                            # DirectX 12 novo, para o Windows 10
```

**3. Configurar**

```powershell
copy victory.example.toml victory.toml
notepad victory.toml        # em game_dir, a pasta da sua cópia do jogo (fora da pasta do projeto)
```

**4. Ajustar o Windows (uma vez)**

- **Arquivo de paginação:** *Sistema > Configurações avançadas do sistema >
  Desempenho > Avançado > Memória virtual*. Desmarque "Gerenciar
  automaticamente", escolha o F:, *Tamanho personalizado*, inicial 16384 e
  máximo 49152, e reinicie.
- **Cache de shaders da NVIDIA:** Painel de Controle da NVIDIA > *Gerenciar
  configurações 3D* > *Tamanho do cache de sombreador*: 10 GB.

**5. Conferir a placa**

```powershell
target\diag\victory.exe gpu
```

Tem que aparecer shader model 6.6 ou mais e "directly indexed heaps: yes".

**6. Traduzir os shaders (uma vez, demora minutos) e jogar**

```powershell
scripts\play-1060.ps1 -Shaders <os arquivos de shaders da sua cópia>
scripts\play-1060.ps1 -Exe <o executável da sua cópia>
```

Se estiver pesado: `-Scale 0.5` (resolução interna menor) e `-MipDrop 3`
(texturas mais leves). Para ver fps, VRAM e RAM na tela: `-Hud` (F3 mostra e
esconde). Se algo der errado: `-Safe`. Caches e saves ficam em
`F:\Sistema\Projetos\victory-data`. Feche o navegador antes de jogar.

**Para atualizar depois:** `git pull` e o `cargo build` do passo 2 de novo.

Mais detalhes e o porquê de cada ajuste: [docs/gtx1060.md](docs/gtx1060.md).
Os nomes internos (`victory.exe`, as variáveis `VICTORY_...`, as crates) ficaram
como estão: mudar quebraria a compatibilidade com as correções que vêm dos
outros projetos.

## Testes no jogo

**Ainda não foram feitos testes com o jogo.** Os testes automáticos rodam com
`cargo test --workspace --locked` e, a cada push, no GitHub Actions (Windows).
O que já foi conferido:

| Data | PC | O que foi testado | Resultado |
|---|---|---|---|
| 26/11/2026 | GTX 1060 6 GB, Ryzen 7 2700, 32 GB, Windows 10 | Compilação no PC | Compila |
| 26/11/2026 | GTX 1060 6 GB, Windows 10 | `victory gpu` com o Agility SDK | Shader model 6.8, directly indexed heaps: sim |
| 26/11/2026 | (nuvem) | Testes do tradutor de shaders | 215/215 tests passing |
| 26/11/2026 | GitHub Actions (Windows, placa WARP) | `cargo test --workspace` | 371 passam, 1 falha: `depth_bounds_pipeline_loads_and_filters_draws`, do código base (depth bounds na placa de software), sem relação com as mudanças da 1060 |
| 26/11/2026 | GitHub Actions (Windows) | Compilação do .zip | Compila |

Esta tabela é atualizada a cada teste com o jogo: se abre, até onde vai, quantos
quadros por segundo, o que aparece errado e com quais opções.

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
- **Pull requests abertos no [carlm125/cool_fox](https://github.com/carlm125/cool_fox)**:
  Yoshi-233 (decodificação de duas instruções do console) e mattkajorinc
  (testes de ondas de 32).
- **[nvidia-3070-VI](https://github.com/grimygunnz/nvidia-3070-VI)**, de
  grimygunnz e codec-d: mais cópias de buffers na placa.
- **[victory-gtx1660](https://github.com/Impre-visible/victory-gtx1660)**, de
  Impre-visible: texturas começando num nível menor, medidas numa placa de 6 GB.
- **[VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**, de
  Belobug: o painel de desempenho (`-Hud`), o limite automático de texturas
  pela memória da placa, a proteção contra o compilador do driver travar, o
  envio de texturas antes de cada submissão (menos tremor) e a ponte do
  FidelityFX FSR para o PC (portada por ele do
  [DexNom/VIctory](https://github.com/DexNom/VIctory)).
- **[VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)**, de
  trustno1-coder: o limite de descritores da NVIDIA e o fim rápido dos raios.

O histórico do git foi mantido: cada commit trazido de outro projeto diz de
onde veio e quem fez.

## Licença

GPL-3.0-or-later, a mesma do VIctory. Veja [LICENSE](LICENSE). Por ser um
trabalho derivado, a licença exige manter o arquivo LICENSE, os avisos de
copyright e o crédito aos autores originais.
