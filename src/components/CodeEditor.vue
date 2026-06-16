<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import { Compartment, EditorState } from "@codemirror/state";
import {
  EditorView,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { StreamLanguage, defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { powerShell } from "@codemirror/legacy-modes/mode/powershell";
import { oneDark } from "@codemirror/theme-one-dark";

// `lang` is the active shell config's stable name (e.g. "powershell" / "cmd" /
// "zshrc"); used to pick syntax highlighting so Windows PowerShell isn't shown
// with POSIX-shell colors.
const props = defineProps<{ modelValue: string; dark: boolean; lang?: string }>();
const emit = defineEmits<{ "update:modelValue": [value: string] }>();

const host = ref<HTMLDivElement | null>(null);
const view = shallowRef<EditorView>();
const themeComp = new Compartment();
const langComp = new Compartment();

// cmd (batch) has no good legacy mode → leave plain; zsh/bash/sh → shell.
function langExt(lang?: string) {
  if (lang === "powershell") return [StreamLanguage.define(powerShell)];
  if (lang === "cmd") return [];
  return [StreamLanguage.define(shell)];
}

const baseTheme = EditorView.theme({
  "&": { height: "100%", fontSize: "13px", backgroundColor: "transparent" },
  ".cm-scroller": { fontFamily: "var(--font-mono)", lineHeight: "1.65" },
  ".cm-content": { padding: "12px 0" },
  ".cm-gutters": { backgroundColor: "transparent", border: "none" },
  "&.cm-focused": { outline: "none" },
});

function themeExt(dark: boolean) {
  return dark
    ? oneDark
    : [
        syntaxHighlighting(defaultHighlightStyle),
        EditorView.theme({
          ".cm-gutters": { color: "var(--c-text-3)" },
          ".cm-activeLine": { backgroundColor: "rgba(15,23,38,0.04)" },
          ".cm-activeLineGutter": { backgroundColor: "transparent" },
          ".cm-cursor": { borderLeftColor: "var(--c-text)" },
        }),
      ];
}

onMounted(() => {
  view.value = new EditorView({
    parent: host.value!,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        highlightActiveLineGutter(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        EditorView.lineWrapping,
        langComp.of(langExt(props.lang)),
        baseTheme,
        themeComp.of(themeExt(props.dark)),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) emit("update:modelValue", u.state.doc.toString());
        }),
      ],
    }),
  });
});

onBeforeUnmount(() => view.value?.destroy());

// External value changes (reload / file switch) → replace the document.
watch(
  () => props.modelValue,
  (val) => {
    const v = view.value;
    if (v && val !== v.state.doc.toString()) {
      v.dispatch({ changes: { from: 0, to: v.state.doc.length, insert: val } });
    }
  },
);

// Theme toggle → reconfigure only the theme compartment.
watch(
  () => props.dark,
  (d) => view.value?.dispatch({ effects: themeComp.reconfigure(themeExt(d)) }),
);

// Active file switch (e.g. PowerShell → cmd) → reconfigure the language.
watch(
  () => props.lang,
  (l) => view.value?.dispatch({ effects: langComp.reconfigure(langExt(l)) }),
);
</script>

<template>
  <div ref="host" class="h-full text-left" />
</template>
