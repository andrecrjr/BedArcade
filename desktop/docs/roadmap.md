# Próximos passos — desktop controlado por gamepad

> Para conhecer o funcionamento atual, consulte [system-overview.md](system-overview.md).

## Objetivo de conclusão

Operar o desktop pelo controle em Windows e Linux: mover o ponteiro, enviar setas e atalhos, digitar com um teclado visual próprio sem perder o foco do aplicativo e trocar/acionar janelas e funções do sistema.

“Completo” significa cobrir uma matriz publicada de sistemas e aplicativos, incluindo as diferenças de entrada e permissões entre os sistemas.

## Estado atual

- [x] Pasta Rust independente da extensão.
- [x] Teclado visual próprio com layout QWERTY e seleção destacada por gamepad.
- [x] R3 abre e fecha o teclado visual sem mudar o foco do aplicativo ativo.
- [x] Windows e Linux/X11 usam viewport transparente, sem ativação, no topo e com mouse pass-through.
- [x] Wayland usa camada `wlr-layer-shell` sem foco de teclado e com região de mouse vazia.
- [x] O painel mostra os comandos do mouse e do teclado visual.
- [x] Leitura comum por SDL3 no Linux e Windows, com perfis de diversas famílias e eventos em segundo plano.
- [x] Seleção de dispositivo e importação/persistência de perfis SDL para controles desconhecidos detectados.
- [x] Conversão de analógicos e botões em movimento, clique/arrasto, rolagem, setas, Enter, Escape, Super/Windows e Alt+Tab.
- [x] Pausa por Start, liberação de cliques ao desconectar e retomada após centralizar/soltar o controle.
- [ ] Concluir a validação física de controle de outros aplicativos e janelas.

## Corte atual: mouse, teclado e teclado visual próprio

Esta entrega prioriza a entrada por mouse e teclado e o teclado visual. A navegação por itens acessíveis foi retirada do corte atual; o direcional envia setas ao aplicativo. A validação física e a matriz de compatibilidade continuam pendentes. A digitação básica está ligada ao foco atual, mas precisa de validação física e de suporte a layouts Linux não US.

- [x] Painel com conexão, pausa, velocidade e zona morta.
- [x] Loop de controle independente do foco da janela do app.
- [x] Bandeja com abrir/recolher painel e encerramento explícito; fechar o painel mantém o serviço ativo.
- [x] Validar recolher/reabrir e encerrar pela bandeja no COSMIC/Wayland sem reiniciar o controle.
- [x] Adaptadores de entrada nativa para Linux e Windows.
- [x] Teclado próprio navegável, com letras, números, símbolos, Shift, Tab, Enter, Esc, espaço e Backspace.
- [x] Em Wayland, teclado sem foco e região de ponteiro vazia para preservar foco, caret e mouse.
- [x] Testes da lógica de movimento, clique, pausa, desconexão, atalhos e repetição.
- [x] Classificação de mouse/teclado virtual pelo udev e testes da seleção/persistência de perfis.
- [x] Leitura física do 8BitDo pela SDL3 e importação de perfil para um controle virtual desconhecido.
- [x] Checagem, lint e build release Windows x64 com SDL3 incorporada; pacote ZIP criado.
- [ ] Criar assistente visual para aprender botões/eixos e calibrar controles desconhecidos sem importar texto.
- [ ] Validar Xbox, DualShock/DualSense, Switch Pro e genéricos físicos por USB e Bluetooth nos dois sistemas.
- [x] Validar build/link de SDL3 no executável GUI x64.
- [ ] Validar o teclado visual e a preservação do foco com controle físico no Windows 11.
- [x] Reempacotar o `.exe` Windows com o teclado visual atualizado usando MinGW x64.
- [ ] Validar mover, clicar, arrastar, rolar e abrir um programa no COSMIC/Wayland com o 8BitDo Ultimate 2 conectado.
- [ ] Validar em Windows, Linux/X11 e outros compositores Wayland.

As etapas completas abaixo só terminam quando seus critérios de validação forem atendidos.

## Etapas

### 1. Fechar a matriz de suporte e escolher a entrada do controle

- [ ] Definir as versões mínimas de Windows e os ambientes Linux a validar: Wayland e X11, incluindo os compositores disponíveis para teste.
- [x] Comparar bibliotecas e APIs nativas: SDL3 escolhida com o usuário para ampliar famílias e perfis, substituindo evdev direto/XInput exclusivo.
- [x] Registrar a decisão e os limites atuais no README; validar fisicamente os modelos da matriz continua pendente.
- [ ] Definir mapeamento inicial, botão para abrir o menu do app, troca de modo e recuperação quando o controle desconectar. Deixar os botões configuráveis; avaliar L3 para abrir o menu Iniciar sem fixá-lo como padrão antes do teste.

**Pronto quando:** o app detecta, identifica e volta a detectar um controle removido/reconectado nos sistemas-alvo.

### 2. Normalizar comandos e controlar a interface do app

- [ ] Criar eventos de domínio independentes da biblioteca: direções, eixos, confirmar, voltar, ombro, clique e botão pressionado/liberado.
- [ ] Tratar deadzone, aceleração, repetição, diagonais e diferença entre toque e pressão longa.
- [x] Conectar o gamepad ao teclado visual próprio; preservar o foco/caret e pausar o movimento do ponteiro enquanto ele está aberto.
- [ ] Permitir escolher controle, ver conexão e editar mapeamentos sem usar teclado ou mouse.
- [ ] Adicionar pausa/retomada e uma ação de saída sempre acessível pelo controle.

**Pronto quando:** é possível abrir e operar todas as telas do próprio Gamepad Desktop só pelo gamepad, incluindo reconexão.

### 3. Criar os adaptadores nativos de cada sistema

- [ ] Fazer uma prova de conceito para obter a posição do campo de texto, enviar entrada de ponteiro/teclado e enumerar janelas em Windows, Linux/Wayland e Linux/X11.
- [ ] Comparar as APIs e as bibliotecas Rust disponíveis antes de definir os contratos comuns. Manter detalhes específicos do sistema nos adaptadores.
- [ ] Expor capacidades disponíveis e indisponíveis por sessão, com mensagens de erro úteis e sem travar o restante do app.
- [ ] Documentar permissões, prompts do sistema, dependências de runtime e limitações por compositor.

**Pronto quando:** há uma checagem mínima por sistema para obter a posição do campo de texto e enviar entrada de mouse/teclado, com falhas explicadas.

### 4. Navegação por itens (fora do escopo atual)

A navegação por itens acessíveis foi retirada. O direcional volta a enviar setas ao aplicativo ativo, que controla o próprio foco.

### 5. Implementar o modo mouse

- [ ] Mover o ponteiro pelo analógico com deadzone, velocidade ajustável e precisão para movimentos pequenos.
- [ ] Mapear confirmar para clique esquerdo; permitir clique direito, pressionar/soltar, arrastar, rolagem e movimento entre monitores.
- [ ] Implementar uma forma clara de iniciar e encerrar arrasto e de recuperar o ponteiro se o controle for desconectado durante a ação.
- [ ] Implementar entrada por sistema: avaliar `SendInput` no Windows; no Wayland, avaliar o portal Remote Desktop/EIS e seu fluxo de autorização; manter caminho próprio para X11.
- [ ] Validar mover/arrastar janelas e as limitações de cada compositor.

**Pronto quando:** ações comuns com ponteiro funcionam nos três ambientes-alvo e o usuário consegue sair de qualquer estado de clique/arrasto.

### 6. Digitar com o teclado visual próprio

- [x] Remover os acionadores do OSK nativo e a dependência de configurações específicas do COSMIC/Plasma.
- [x] Compartilhar o modelo do teclado e a navegação por gamepad entre os hosts visuais.
- [x] Windows: abrir uma janela transparente sem ativação, no topo e com mouse pass-through; enviar caracteres Unicode com Enigo.
- [x] Linux/X11: abrir o teclado visual sem ativação e enviar teclas via `uinput`.
- [x] Linux/Wayland: usar `wlr-layer-shell`, `KeyboardInteractivity::None` e região de mouse vazia.
- [x] R3 abre/fecha; direcional navega; A digita; B apaga; X envia Enter; Y envia Esc; L3 envia espaço; L1 envia Tab; R1 alterna Shift.
- [ ] Detectar foco em campo de texto e abrir o teclado automaticamente por APIs de acessibilidade, sem tirar o foco do campo.
- [ ] Validar foco, caret, cursor do mouse e campos de senha na matriz de aplicativos e sistemas.
- [ ] Linux: obter o mapa de teclado da sessão para renderizar e enviar caracteres de acordo com o layout ativo, incluindo acentos.
- [ ] Investigar uma alternativa para GNOME Wayland, que não oferece `wlr-layer-shell`, sem tomar foco nem depender de uma implementação nativa diferente.

**Pronto quando:** o teclado aparece sem roubar foco, o mouse passa por baixo e a tecla escolhida chega ao cursor de texto em todas as combinações suportadas.

### Atalhos a avaliar com o usuário

Estas combinações são propostas; ainda não estão implementadas:

- [ ] LT / L2: precisão para apontar em alvos pequenos.
- [ ] LT + LB / L1: Shift+Tab para voltar ao campo anterior.
- [ ] Select / View / −: menu rápido acessível pelo controle, com copiar, colar, volume e ações de janela.
- [ ] No menu rápido: voltar/avançar no navegador, mostrar desktop e maximizar/restaurar.
- [ ] Tornar as combinações configuráveis e mostrar possíveis conflitos antes de salvar.

### 7. Adicionar janelas e ações do sistema

- [ ] Listar janelas e alternar entre elas por gamepad, preservando modo e posição de foco.
- [ ] Oferecer voltar à janela anterior e retornar ao campo de texto após fechar o teclado virtual.
- [ ] Adicionar ações configuráveis para menu Iniciar, alternância de janela, minimizar, maximizar e fechar.
- [ ] Permitir mostrar/ocultar o overlay sem interromper o app que está sendo controlado.
- [ ] Validar comportamento em múltiplos monitores, áreas de trabalho e diálogos modais.

**Pronto quando:** é possível trocar de janela, usar ações do sistema e voltar ao contexto anterior sem mouse/teclado.

### 8. Finalizar a experiência e distribuir

- [ ] Criar configuração de sensibilidade, deadzone, repetição, layouts, mapeamentos e perfis por aplicativo.
- [ ] Mostrar estado do controle, modo ativo, janela-alvo e permissões de entrada sem cobrir conteúdo importante.
- [ ] Criar primeira execução guiada, recuperação de permissões e instruções de compatibilidade.
- [ ] Definir instalação/atualização para Windows e Linux e opção de iniciar com a sessão.
- [ ] Criar CI de Rust para Windows e Linux; validar Linux em Wayland e X11.
- [ ] Fazer testes de integração com controles físicos e atualizar a matriz pública de aplicativos/sistemas.

**Pronto quando:** uma pessoa instala o app, conecta o controle, concede permissões necessárias e conclui o fluxo sem configurar o projeto manualmente.

## Critério de qualidade para cada etapa

- [ ] Seguir RED → GREEN para cada comportamento observável.
- [ ] Rodar `cargo fmt`, `cargo clippy` e testes da etapa.
- [ ] Anotar sistemas, controle físico e aplicativos usados na validação.
- [ ] Atualizar este arquivo e o README ao concluir uma etapa.

## Referências técnicas

- [UI Automation — visão geral e clientes](https://learn.microsoft.com/en-us/windows/win32/winauto/entry-uiautocore-overview)
- [UI Automation — padrões de controle](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-controlpatternsoverview)
- [Windows `SendInput`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendinput)
- [AT-SPI2](https://wiki.freedesktop.org/www/Accessibility/AT-SPI2/)
- [Portal XDG Remote Desktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html)
- [Comportamento atual da extensão: navegação espacial](../../lib/spatial.ts)
- [Comportamento atual da extensão: teclado flutuante](../../lib/floating-keyboard.ts)
