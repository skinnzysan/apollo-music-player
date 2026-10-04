#!/bin/bash
sed -i -e '/let (ch, color) = if total_sublevels >= row_min_sub + 8 {/c\
            let (ch, color) = if total_sublevels >= row_min_sub + 8 {\
                let col = match app.config.visualizer_color {\
                    crate::config::VisualizerColorMode::Rainbow => {\
                        let colors = [Color::Red, Color::Yellow, Color::Green, Color::Cyan, Color::Blue, Color::Magenta];\
                        colors[b_idx % colors.len()]\
                    }\
                    crate::config::VisualizerColorMode::Solid => Color::Reset,\
                };\
                (UNICODE_BLOCKS[8], col)\
            } else if total_sublevels > row_min_sub {\
                let sub = total_sublevels - row_min_sub;\
                let col = match app.config.visualizer_color {\
                    crate::config::VisualizerColorMode::Rainbow => {\
                        let colors = [Color::Red, Color::Yellow, Color::Green, Color::Cyan, Color::Blue, Color::Magenta];\
                        colors[b_idx % colors.len()]\
                    }\
                    crate::config::VisualizerColorMode::Solid => Color::Reset,\
                };\
                (UNICODE_BLOCKS[sub.min(8)], col)' src/ui.rs
