import { AppBar, Box, Button, Container, CssBaseline, Link, Stack, Toolbar, Typography } from '@mui/material'
import type { NextPage } from 'next'
import Head from 'next/head'
import Image from 'next/image'
import { createTheme, ThemeProvider } from '@mui/material/styles';
import React from 'react'

const theme = createTheme({
  palette: {
    primary: {
      main: '#263238',
      light: '#4f5b62',
      dark: '#000a12',
      contrastText: '#fefefe',
    },
    secondary: {
      main: '#01579b',
      light: '#4f83cc',
      dark: '#002f6c',
      contrastText: '#fefefe',
    },
  },
});

// Based on a MUI's template:
// https://github.com/mui-org/material-ui/tree/master/docs/index.html/src/pages/getting-started/templates/album
const Home: NextPage = () => {
  return (
    <div>
      <Head>
        <title>Kerla</title>
        <meta name="description" content="A new operating system in Rust, with Linux ABI compatibility." />
      </Head>

      <ThemeProvider theme={theme}>
        <CssBaseline />
        <AppBar position="relative" elevation={0}>
          <Toolbar>
            <Typography variant="h6" color="inherit" noWrap sx={{ flexGrow: 1 }}>
              Kerla
            </Typography>

            <Button color="inherit" href="/docs/index.html">Docs</Button>
            <Button color="inherit" href="https://github.com/nuta/kerla">GitHub</Button>
            <Button color="inherit" href="https://discord.gg/6Pu4ujpp6h">Discord</Button>
          </Toolbar>
        </AppBar>

        <main>
          <Box sx={{ bgcolor: 'background.paper', pt: 8, pb: 6 }}>
            <Container maxWidth="sm">
              <Typography
                component="h1"
                variant="h2"
                align="center"
                color="text.primary"
                gutterBottom
              >
                Kerla
              </Typography>
              <Typography variant="h5" align="center" color="text.secondary" paragraph>
                A new operating system with Linux ABI compatibility, written in Rust.
              </Typography>
              <Stack
                sx={{ pt: 4 }}
                direction="row"
                spacing={2}
                justifyContent="center"
              >
                <Button variant="contained" href="/docs/quickstart.html">Quickstart</Button>
                <Button variant="outlined" href="https://github.com/nuta/kerla">GitHub</Button>
              </Stack>

              <Box sx={{ pt: 4 }}>
                <Image src="/screenshot.png" width="640" height="412"
                  alt="A screenshot of Kerla. It can run Linux binaries like Busybox and curl!" />
              </Box>
            </Container>
          </Box>
        </main>
        <footer>
          <Box sx={{ bgcolor: 'background.paper', p: 6 }} component="footer">
            <Typography
              variant="subtitle1"
              align="center"
              color="text.secondary"
              component="p"
            >
              Made by Kerla Authors.<br />
              <Link href="/docs/index.html" color="inherit" sx={{ mr: "1rem" }}>Docs</Link>
              <Link href="https://github.com/nuta/kerla" color="inherit" sx={{ mr: "1rem" }}>GitHub</Link>
              <Link href="https://discord.gg/6Pu4ujpp6h" color="inherit">Discord</Link>
            </Typography>
          </Box>
        </footer>
      </ThemeProvider>
    </div>
  )
}

export default Home
