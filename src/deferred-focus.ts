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

type NativeWindowFocus = {
  setFocus: () => Promise<void>;
};

// Request native activation during a user gesture, without blocking the press.
export function focusWindowOnPress(appWindow: NativeWindowFocus | undefined) {
  if (!appWindow) return;
  void appWindow.setFocus().catch(() => {
    // The webview can still receive the press if the window manager refuses focus.
  });
}
