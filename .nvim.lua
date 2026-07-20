vim.lsp.config('rust_analyzer', {
  settings = {
    ['rust-analyzer'] = {
      cargo = {
        features = 'all',
      },
      check = {
        features = 'all',
      },
    },
  },
})
