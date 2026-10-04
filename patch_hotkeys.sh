#!/bin/bash
sed -i -e 's/Span::styled("\[ Spacja \]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))/Span::styled(" Spacja ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))/g' src/ui.rs
sed -i -e 's/Span::styled("\[ z \/ x \]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))/Span::styled(" z \/ x ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))/g' src/ui.rs
sed -i -e 's/Span::styled("\[ < \/ > \]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))/Span::styled(" < \/ > ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))/g' src/ui.rs
sed -i -e 's/Span::styled("\[ s \]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))/Span::styled(" s ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))/g' src/ui.rs
sed -i -e 's/Span::styled("\[ l \]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))/Span::styled(" l ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))/g' src/ui.rs
sed -i -e 's/Span::styled("\[ + \/ - \]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))/Span::styled(" + \/ - ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))/g' src/ui.rs
sed -i -e 's/Span::styled("\[ \/ \]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))/Span::styled(" \/ ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))/g' src/ui.rs
sed -i -e 's/Span::styled("\[ q \]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))/Span::styled(" q ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))/g' src/ui.rs
