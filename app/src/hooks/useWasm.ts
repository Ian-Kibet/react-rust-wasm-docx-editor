import { useState, useEffect } from 'react';
import { initWasm } from '../wasm/loader';

export function useWasm() {
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    initWasm()
      .then(() => setReady(true))
      .catch((e) => setError(e.message || 'Failed to initialize WASM'));
  }, []);

  return { ready, error };
}
