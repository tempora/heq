using System;

namespace Heq
{
    public sealed class Scope : IDisposable
    {
        private Action _end;

        public Scope(Action end) => _end = end;

        public void Dispose()
        {
            var end = _end;
            _end = null;
            end?.Invoke();
        }
    }
}
