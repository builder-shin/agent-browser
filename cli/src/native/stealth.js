(function() {
  'use strict';

  // === IDEMPOTENCY GUARD ===
  if (window.__stealth_applied) return;
  window.__stealth_applied = true;

  // === EVASION: navigator.webdriver ===
  // Defense-in-depth: --disable-blink-features=AutomationControlled handles this at Chrome level,
  // but we also patch it in JS for extra safety.
  Object.defineProperty(navigator, 'webdriver', {
    get: () => undefined,
    configurable: true
  });
  // Also delete it if it exists as own property
  if (navigator.hasOwnProperty && navigator.hasOwnProperty('webdriver')) {
    delete navigator.webdriver;
  }

  // === EVASION: HeadlessChrome UA ===
  // Only patch if the UA contains "HeadlessChrome" to respect user-specified --user-agent
  if (navigator.userAgent && navigator.userAgent.includes('HeadlessChrome')) {
    const originalUA = navigator.userAgent;
    const patchedUA = originalUA.replace(/HeadlessChrome/g, 'Chrome');
    Object.defineProperty(navigator, 'userAgent', {
      get: () => patchedUA,
      configurable: true
    });
    // Also patch userAgentData if available
    if (navigator.userAgentData && navigator.userAgentData.brands) {
      const originalBrands = navigator.userAgentData.brands;
      const patchedBrands = originalBrands.map(b => ({
        ...b,
        brand: b.brand.replace(/HeadlessChrome/g, 'Chrome')
      }));
      Object.defineProperty(navigator.userAgentData, 'brands', {
        get: () => patchedBrands,
        configurable: true
      });
    }
  }

  // === EVASION: navigator.plugins ===
  // Spoof a realistic plugin array (Chrome PDF Plugin, Chrome PDF Viewer, Native Client)
  const makePlugin = (name, description, filename, mimeType) => {
    const plugin = Object.create(Plugin.prototype);
    Object.defineProperties(plugin, {
      name: { get: () => name, enumerable: true },
      description: { get: () => description, enumerable: true },
      filename: { get: () => filename, enumerable: true },
      length: { get: () => 1, enumerable: true },
      0: { get: () => ({ type: mimeType, suffixes: '', description, enabledPlugin: plugin }) }
    });
    return plugin;
  };

  try {
    const plugins = [
      makePlugin('Chrome PDF Plugin', 'Portable Document Format', 'internal-pdf-viewer', 'application/x-google-chrome-pdf'),
      makePlugin('Chrome PDF Viewer', '', 'mhjfbmdgcfjbbpaeojofohoefgiehjai', 'application/pdf'),
      makePlugin('Native Client', '', 'internal-nacl-plugin', 'application/x-nacl')
    ];

    const pluginArray = Object.create(PluginArray.prototype);
    plugins.forEach((p, i) => {
      Object.defineProperty(pluginArray, i, { get: () => p, enumerable: true });
      Object.defineProperty(pluginArray, p.name, { get: () => p });
    });
    Object.defineProperty(pluginArray, 'length', { get: () => plugins.length, enumerable: true });
    pluginArray.item = (i) => plugins[i] || null;
    pluginArray.namedItem = (name) => plugins.find(p => p.name === name) || null;
    pluginArray.refresh = () => {};
    pluginArray[Symbol.iterator] = function* () { yield* plugins; };

    Object.defineProperty(navigator, 'plugins', {
      get: () => pluginArray,
      configurable: true
    });
  } catch (e) { /* fail silently */ }

  // === EVASION: navigator.languages ===
  if (!navigator.languages || navigator.languages.length === 0) {
    Object.defineProperty(navigator, 'languages', {
      get: () => ['en-US', 'en'],
      configurable: true
    });
  }
  if (!navigator.language) {
    Object.defineProperty(navigator, 'language', {
      get: () => 'en-US',
      configurable: true
    });
  }

  // === EVASION: navigator.permissions ===
  // Override Permissions.prototype.query to return "prompt" for notifications
  // (headless Chrome returns "denied" by default, which is detectable)
  if (typeof Permissions !== 'undefined' && Permissions.prototype) {
    const originalQuery = Permissions.prototype.query;
    Permissions.prototype.query = function(parameters) {
      if (parameters && parameters.name === 'notifications') {
        return Promise.resolve({ state: (typeof Notification !== 'undefined' && Notification.permission) || 'prompt', onchange: null });
      }
      return originalQuery.call(this, parameters);
    };
  }

  // === EVASION: WebGL vendor/renderer ===
  // Override getParameter for UNMASKED_VENDOR/RENDERER to return realistic Intel GPU strings
  const getParameterProxyHandler = {
    apply: function(target, thisArg, args) {
      const param = args[0];
      const UNMASKED_VENDOR_WEBGL = 0x9245;
      const UNMASKED_RENDERER_WEBGL = 0x9246;
      if (param === UNMASKED_VENDOR_WEBGL) return 'Intel Inc.';
      if (param === UNMASKED_RENDERER_WEBGL) return 'Intel Iris OpenGL Engine';
      return Reflect.apply(target, thisArg, args);
    }
  };

  try {
    const getParameterOrig = WebGLRenderingContext.prototype.getParameter;
    WebGLRenderingContext.prototype.getParameter = new Proxy(getParameterOrig, getParameterProxyHandler);
  } catch (e) { /* fail silently */ }

  try {
    const getParameterOrig2 = WebGL2RenderingContext.prototype.getParameter;
    WebGL2RenderingContext.prototype.getParameter = new Proxy(getParameterOrig2, getParameterProxyHandler);
  } catch (e) { /* fail silently */ }

  // === EVASION: window.chrome ===
  // === EVASION: chrome.csi/chrome.loadTimes ===
  // Inject window.chrome runtime object with csi and loadTimes
  if (!window.chrome) {
    window.chrome = {};
  }
  if (!window.chrome.runtime) {
    window.chrome.runtime = {
      connect: function() { return { onMessage: { addListener: function() {} }, postMessage: function() {}, onDisconnect: { addListener: function() {} } }; },
      sendMessage: function() {}
    };
  }
  if (!window.chrome.csi) {
    window.chrome.csi = function() {
      return {
        startE: Date.now(),
        onloadT: Date.now(),
        pageT: performance.now(),
        tpiE: Date.now()
      };
    };
  }
  if (!window.chrome.loadTimes) {
    window.chrome.loadTimes = function() {
      const perfEntry = performance.getEntriesByType('navigation')[0] || {};
      return {
        requestTime: perfEntry.requestStart ? perfEntry.requestStart / 1000 : Date.now() / 1000,
        startLoadTime: perfEntry.startTime ? perfEntry.startTime / 1000 : Date.now() / 1000,
        commitLoadTime: perfEntry.responseStart ? perfEntry.responseStart / 1000 : Date.now() / 1000,
        finishDocumentLoadTime: perfEntry.domContentLoadedEventEnd ? perfEntry.domContentLoadedEventEnd / 1000 : Date.now() / 1000,
        finishLoadTime: perfEntry.loadEventEnd ? perfEntry.loadEventEnd / 1000 : Date.now() / 1000,
        firstPaintTime: 0,
        firstPaintAfterLoadTime: 0,
        navigationType: 'Other',
        wasFetchedViaSpdy: false,
        wasNpnNegotiated: false,
        npnNegotiatedProtocol: 'unknown',
        wasAlternateProtocolAvailable: false,
        connectionInfo: 'h2'
      };
    };
  }

  // === EVASION: iframe.contentWindow ===
  // Ensure contentWindow is properly accessible for same-origin iframes
  try {
    const originalHTMLIFrameElement = HTMLIFrameElement.prototype.__lookupGetter__('contentWindow');
    if (originalHTMLIFrameElement) {
      Object.defineProperty(HTMLIFrameElement.prototype, 'contentWindow', {
        get: function() {
          const result = originalHTMLIFrameElement.call(this);
          if (result && result.navigator) {
            // Ensure the iframe's navigator also has patched webdriver
            try {
              Object.defineProperty(result.navigator, 'webdriver', {
                get: () => undefined,
                configurable: true
              });
            } catch (e) { /* cross-origin, ignore */ }
          }
          return result;
        },
        configurable: true
      });
    }
  } catch (e) { /* fail silently */ }

  // === EVASION: navigator.hardwareConcurrency ===
  // Normalize to 4 if less than 2 (headless often reports 1)
  if (navigator.hardwareConcurrency < 2) {
    Object.defineProperty(navigator, 'hardwareConcurrency', {
      get: () => 4,
      configurable: true
    });
  }

})();
