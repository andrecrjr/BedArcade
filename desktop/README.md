# Gamepad Navigator Desktop

Use o controle para navegar pelo computador, mover o ponteiro e digitar sem tirar o foco do aplicativo que você está usando.

O app tem três recursos principais: modo mouse, navegação pelos itens que cada aplicativo disponibiliza para acessibilidade e um teclado visual próprio. Ele continua funcionando quando o painel é recolhido para a bandeja do sistema.

> O desktop está em desenvolvimento. A navegação depende do suporte de cada aplicativo, e ainda falta validar o uso com controles físicos em toda a matriz de Windows e Linux.

![Prévia do teclado visual](keyboard-preview.png)

## Começar

No Windows, baixe e extraia o [pacote x64](dist/GamepadNavigator-Windows-x64.zip), conecte o controle e abra `GamepadNavigator.exe`. O painel mostra quando o controle foi reconhecido e quais comandos estão disponíveis.

No Linux, ainda não há instalador. As instruções para executar o app e as permissões necessárias estão no [guia do sistema](docs/system-overview.md#como-iniciar-no-linux).

## Comandos principais

| Controle                             | Ação                                                   |
| ------------------------------------ | ------------------------------------------------------ |
| Analógico esquerdo                   | Mover o ponteiro ou navegar pelos itens                |
| Analógico direito, cima/baixo        | Rolar a página                                         |
| Direcional                           | Enviar setas ou mover o foco                           |
| Botão inferior — A / ✕ / B no Switch | Clicar; no teclado visual, digitar a tecla selecionada |
| L2 + R2                              | Alternar entre mouse e navegação por itens             |
| R3 — pressionar o analógico direito  | Abrir ou fechar o teclado visual                       |
| Start / Menu / +                     | Pausar ou retomar                                      |

Os nomes e comandos completos aparecem no painel, incluindo os equivalentes de Xbox, PlayStation e Switch. A seção de ajustes permite escolher o controle, ajustar velocidade e personalizar o teclado.

## O que saber antes de usar

- A navegação por itens funciona quando o aplicativo publica seus controles para acessibilidade. Quando isso não acontece, use o modo mouse.
- O teclado visual usa o layout QWERTY em inglês. No Linux, símbolos e acentos podem variar conforme o layout ativo.
- Em algumas sessões Wayland, incluindo o GNOME Wayland, o teclado visual não está disponível.
- O pacote Windows existe, mas o uso com controle físico ainda precisa de validação em Windows 11. A compatibilidade em Linux também depende do compositor e das permissões de entrada.

## Documentação

- [Como o sistema funciona](docs/system-overview.md): modos, fluxo do controle, diferenças entre sistemas, limites e execução.
- [Pesquisa do teclado visual](docs/keyboard-research.md): decisões de layout, foco e posicionamento.
- [Roadmap](docs/roadmap.md): trabalho concluído e validações pendentes.
