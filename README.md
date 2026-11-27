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
| Testes automáticos no Windows | 🟡 | 388 passam (437 com diagnostics), 1 falha do código base (ver Testes no jogo) |
| Testes do tradutor de shaders | ✅ | 215 testes passando (`cargo test -p victory-shader`) |
| Clippy sem avisos | ✅ | Windows, com e sem diagnóstico |
| Agility SDK no Windows 10 | ✅ | `victory gpu` na GTX 1060: shader model 6.8, directly indexed heaps |
| Mesh shaders como compute (Pascal) | 🟡 | Roda com o jogo; baricêntricas emuladas (a 1060 não tem): recusados de 342 para 69 pipelines. Lento (cada desenho de malha é dispatch, barreira e desenho) |
| Perfil para 6 GB / 32 GB (`play-1060.ps1`) | 🟡 | Feito, ainda não medido |
| Ray tracing | ❌ | Desligado: não funciona numa GTX |
| Rodar com o jogo | 🟡 | Abre, passa da tela de usuário (`-AutoEnter`) e chega ao prólogo, a 1,5 a 4 fps; a placa é o gargalo (ver Testes no jogo) |

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
esconde). Para não precisar apertar Enter na tela de usuário ("Player (A)"):
`-AutoEnter`. Para jogar no teclado e mouse como no GTA V do PC: `-Kbm` (F2 prende o mouse
na câmera; Esc, P ou sair da janela solta). Botões: esquerdo = RT, direito = LT,
meio ou C = R3, Shift = A, Espaço = X, R = B, F = Y, Tab = LB, Q = RB,
Ctrl = L3, E = direcional direita, V = View, Enter continua A nos menus; sensibilidade com
`-More @{VICTORY_MOUSE_SENS='1.5'}` e eixo invertido com
`-More @{VICTORY_MOUSE_INVERT_Y='1'}`. Se algo der errado: `-Safe`. Caches e saves ficam em
`F:\Sistema\Projetos\victory-data`. Feche o navegador antes de jogar.

**Para atualizar depois:** `git pull` e o `cargo build` do passo 2 de novo.

Mais detalhes e o porquê de cada ajuste: [docs/gtx1060.md](docs/gtx1060.md).
Os nomes internos (`victory.exe`, as variáveis `VICTORY_...`, as crates) ficaram
como estão: mudar quebraria a compatibilidade com as correções que vêm dos
outros projetos.

## Testes no jogo

**Primeiro teste com o jogo em 27/11/2026:** abre e chega ao prólogo, mas a 1,5 a 4 fps. Para medir uma rodada:
`scripts\perf-report.ps1 <log> -LastSeconds 60` (fps, tempo na placa e no processador, VRAM, o que tentar;
`-Follow` acompanha ao vivo). Os testes automáticos rodam com
`cargo test --workspace --locked` e, a cada push, no GitHub Actions (Windows).
O que já foi conferido:

| Data | PC | O que foi testado | Resultado |
|---|---|---|---|
| 26/11/2026 | GTX 1060 6 GB, Ryzen 7 2700, 32 GB, Windows 10 | Compilação no PC | Compila |
| 26/11/2026 | GTX 1060 6 GB, Windows 10 | `victory gpu` com o Agility SDK | Shader model 6.8, directly indexed heaps: sim |
| 26/11/2026 | (nuvem) | Testes do tradutor de shaders | 215/215 tests passing |
| 26/11/2026 | GitHub Actions (Windows, placa WARP) | `cargo test --workspace` | 388 passam (437 com diagnostics), 1 falha: `depth_bounds_pipeline_loads_and_filters_draws`, do código base (depth bounds na placa de software), sem relação com as mudanças da 1060. Conferido de novo depois do código do Belobug (b98474b): clippy limpo, zip gerado |
| 26/11/2026 | GitHub Actions (Windows) | Compilação do .zip | Compila |
| 27/11/2026 | GTX 1060 6 GB, Ryzen 7 2700, 32 GB, Windows 10 (19045), projeto e jogo no mesmo SSD | `play-1060.ps1 -Shaders` com a cópia inteira do jogo | 25.787 shaders no cache em 8 min; 198 não traduzem (195 contêineres sem programa de vértice/pixel/compute/malha, 3 com troca de máscara de lanes onde nem todas rodam); 16 arquivos acima de 2 GB pulados |
| 27/11/2026 | GTX 1060 6 GB, Ryzen 7 2700, 32 GB, Windows 10 | `play-1060.ps1 -Exe game_scarlett_master.exe -Hud` (padrão: Series S, 1280x720, MipDrop 2) | **Abre.** Janela 1280x720, 30 fps estáveis (quadro 33,4 ms, GPU 8,8 ms, 11 desenhos por quadro), VRAM 1,3 / 5,1 GB, RAM do processo 5,8 GB (15,9 / 31,9 GB no PC), CPU 24 %, placa 26 %. Para na tela azul de usuário ("Seletor de conta (Y) / Player (A)"): o `XUserAddAsync` entregou o usuário offline "Player" e o jogo espera o botão A (Enter no teclado, com a janela em foco). Sem crash. O executável aparece como `build unknown` (não está na lista de builds conhecidos). Corrigido no `play-1060.ps1`: a variável `$build` colidia com o parâmetro `-Build` (ValidateSet) e dava erro no Windows 10. **Depois do Enter:** passa da tela de usuário e desenha o mundo (uma personagem em 3D com roupa, texturas e cenário), mas a cabeça sai preta com só olhos e dentes, a 2 a 3 fps (~420 ms por quadro, ~1.700 desenhos). 357 pipelines de malha + pixel recusados pelo driver ("Parâmetro incorreto", 0x80070057). Fecha sozinho depois de ~15 min: "a host command list did not close" (0x80004005, comando inválido gravado na lista) |
| 27/11/2026 | GTX 1060 6 GB, Ryzen 7 2700, 32 GB, Windows 10 | O mesmo com `-Safe` e `VICTORY_D3D12_DEBUG=1` | 1,6 a 2 fps (~500 a 640 ms por quadro). 460 pipelines recusados. Fecha depois de ~7 min, bem na hora em que o gameplay ia começar: a placa travou (TDR do driver NVIDIA, eventos nvlddmkm 13/153 e Display 4101; DXGI 0x887A0006, device hung) com uma lista de comandos de malha como compute (desenho indireto) em andamento. A camada de depuração não ligou: falta o recurso opcional "Ferramentas de Gráficos" do Windows |
| 27/11/2026 | GTX 1060 6 GB, Ryzen 7 2700, 32 GB, Windows 10, com "Ferramentas de Gráficos" | Padrão com `-AutoEnter` e `VICTORY_D3D12_DEBUG=1` | Não travou desta vez (fechado por nós). ~1,5 fps (~690 ms por quadro), até ~4.000 desenhos por quadro; cena da prisão com a cabeça e o pescoço pretos, braços sem textura, uma faixa verde enorme na tela. A depuração mostrou a causa dos pipelines recusados: **509 shaders de pixel usam baricêntricas** (`SV_Barycentrics`/`attributeAtVertex`), que a GTX 1060 não tem; 342 pipelines de malha + pixel caíam por isso. Também: 6 compute shaders pedem ondas de 64 (a placa só tem 32) e um estado de recurso errado (`COPY_DEST` lido como SRV) |
| 27/11/2026 | Igual | Depois de emular as baricêntricas (abaixo) e do merge do PR #1 (Belobug), padrão com `-AutoEnter` | **Texturas e peles melhoraram** (relato visual). Pipelines recusados: de 342 a 69. Ainda lento: 2,2 fps (446 ms por quadro), **313 ms deles na placa**, 61 ms preparando no processador, 33 ms copiando de volta; ~3.200 desenhos; VRAM 4,6 de 4,8 GB e **6,3 GB da memória do PC usados pela placa** (lidos pelo barramento). O PC fica pesado (RAM 28 de 32 GB) |
| 27/11/2026 | Igual | `-Scale 0.5 -MipDrop 3 -AutoEnter` | Chega ao "Prologue". 1,5 a 4 fps (228 a 650 ms por quadro, conforme a cena; até ~4.600 desenhos), placa a 100 %, VRAM 4,4 de 4,7 GB e 6,9 GB na memória do PC. Escala e texturas menores quase não mudam: o gargalo não é resolução |

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
- **[kelvaroq](https://github.com/stellarr-studio/kelvaroq)**, de fablinoo: texturas
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
  [DexNom/VIctory](https://github.com/DexNom/VIctory)). Em 27/11 vieram mais
  28 commits do `a-publier` com autores da comunidade: teclado e mouse estilo
  GTA V (zhu1025960209-web), descritores do port 2 sem trabalho dobrado
  (SalmanBarkat), VRAM do painel separada em uso / orçamento / total (IronMan),
  aviso claro quando o Windows recusa os heaps indexados (adrianmjim), o
  `warm-bad.txt` por placa e driver (BlueDragonReal), o perfil sem crash
  (Oğuz Akın, Lux-Designs), o teste de depth bounds na WARP (loyodea), o
  `--warm` usando todos os núcleos (TinkerBell-90), a consulta de tamanho que
  derrubava o compilador (alexdeliu) e outros.
- **[VIctory-NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA)**, de
  trustno1-coder: o limite de descritores da NVIDIA e o fim rápido dos raios.

O histórico do git foi mantido: cada commit trazido de outro projeto diz de
onde veio e quem fez.

## Licença

GPL-3.0-or-later, a mesma do VIctory. Veja [LICENSE](LICENSE). Por ser um
trabalho derivado, a licença exige manter o arquivo LICENSE, os avisos de
copyright e o crédito aos autores originais.
