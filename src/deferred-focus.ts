// GTK must finish dispatching the mouse event before a webview changes focus.
export function createDeferredFocus(restore: () => void) {
  let pending: ReturnType<typeof setTimeout> | undefined;

  function cancel() {
    if (pending !== undefined) clearTimeout(pending);
    pending = undefined;
  }

  return {
    request() {
      cancel();
      pending = setTimeout(() => {
        pending = undefined;
        restore();
      }, 0);
    },
    cancel,
  };
}
