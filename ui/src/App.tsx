import { Component, createSignal, onMount, Show } from 'solid-js';
import Navigation from './components/layout/Navigation';
import ContentArea from './components/layout/ContentArea';
import TableOfContents from './components/layout/TableOfContents';
import Breadcrumbs from './components/layout/Breadcrumbs';

type DocCategory = 'user' | 'contributor';

const App: Component = () => {
  const [currentCategory, setCurrentCategory] = createSignal<DocCategory>('user');
  const [currentSection, setCurrentSection] = createSignal<string | null>(null);
  const [currentPage, setCurrentPage] = createSignal<string | null>('index');
  const [content, setContent] = createSignal<string>('');

  // Handle hash-based routing
  onMount(() => {
    const handleHashChange = () => {
      const hash = window.location.hash;
      
      if (hash === '' || hash === '#' || hash === '#/') {
        setCurrentCategory('user');
        setCurrentSection(null);
        setCurrentPage('index');
        if (window.location.hash !== '#/') {
          window.history.replaceState(null, '', '#/');
        }
      } else if (hash.startsWith('#/user/')) {
        setCurrentCategory('user');
        const path = hash.replace('#/user/', '');
        const parts = path.split('/').filter(p => p);
        if (parts.length >= 2) {
          setCurrentSection(parts[0]);
          setCurrentPage(parts.slice(1).join('/'));
        } else if (parts.length === 1 && parts[0]) {
          setCurrentSection(parts[0]);
          setCurrentPage(null);
        }
      } else if (hash.startsWith('#/contributor/')) {
        setCurrentCategory('contributor');
        const path = hash.replace('#/contributor/', '');
        const parts = path.split('/').filter(p => p);
        if (parts.length >= 2) {
          setCurrentSection(parts[0]);
          setCurrentPage(parts.slice(1).join('/'));
        } else if (parts.length === 1 && parts[0]) {
          setCurrentSection(parts[0]);
          setCurrentPage(null);
        }
      } else {
        setCurrentCategory('user');
        setCurrentSection(null);
        setCurrentPage('index');
        window.history.replaceState(null, '', '#/');
      }
    };

    handleHashChange();
    window.addEventListener('hashchange', handleHashChange);
    
    return () => {
      window.removeEventListener('hashchange', handleHashChange);
    };
  });

  return (
    <div class="min-h-screen bg-[#faf9f7] flex flex-col">
      <header class="bg-white border-b border-[#e5e3df] shadow-sm sticky top-0 z-10" role="banner">
        <div class="max-w-7xl mx-auto">
          <div class="px-6 py-4 flex items-center justify-between">
            <button
              onClick={() => {
                setCurrentCategory('user');
                setCurrentSection(null);
                setCurrentPage('index');
                window.location.hash = '#/';
              }}
              class="text-2xl font-semibold text-[#2d3748] tracking-tight hover:text-[#4a5a4c] transition-colors cursor-pointer"
              aria-label="Go to home page"
            >
              DCops
            </button>
            <nav class="flex gap-3 items-center" aria-label="Main navigation">
              <button
                onClick={() => {
                  setCurrentCategory('user');
                  setCurrentSection(null);
                  setCurrentPage('index');
                  window.location.hash = '#/';
                }}
                class={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                  currentCategory() === 'user'
                    ? 'bg-[#4a5a4c] text-white shadow-sm'
                    : 'bg-[#f1f0ed] text-[#4a5568] hover:bg-[#e5e3df]'
                }`}
              >
                User Docs
              </button>
              <button
                onClick={() => {
                  setCurrentCategory('contributor');
                  window.location.hash = '#/contributor/development/setup';
                }}
                class={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                  currentCategory() === 'contributor'
                    ? 'bg-[#4a5a4c] text-white shadow-sm'
                    : 'bg-[#f1f0ed] text-[#4a5568] hover:bg-[#e5e3df]'
                }`}
              >
                Contributor Docs
              </button>
            </nav>
          </div>
          <div class="px-6 py-2 border-t border-[#e5e3df] bg-[#faf9f7]">
            <Breadcrumbs
              category={currentCategory()}
              section={currentSection()}
              page={currentPage()}
              onNavigate={(category, section, page) => {
                setCurrentCategory(category);
                setCurrentSection(section);
                setCurrentPage(page);
                if (page === 'index' && !section) {
                  window.location.hash = '#/';
                } else {
                  window.location.hash = `#/${category}/${section}${page ? `/${page}` : ''}`;
                }
              }}
            />
          </div>
        </div>
      </header>

      <div class="flex-1 flex">
        <Navigation
          category={currentCategory()}
          currentSection={currentSection()}
          currentPage={currentPage()}
          onNavigate={(category, section, page) => {
            setCurrentCategory(category);
            setCurrentSection(section);
            setCurrentPage(page);
            if (page === 'index' && !section) {
              window.location.hash = '#/';
            } else {
              window.location.hash = `#/${category}/${section}${page ? `/${page}` : ''}`;
            }
          }}
        />
        <ContentArea
          category={currentCategory()}
          section={currentSection()}
          page={currentPage()}
          onContentChange={setContent}
          onNavigate={(category, section, page) => {
            setCurrentCategory(category);
            setCurrentSection(section);
            setCurrentPage(page);
            if (page === 'index' && !section) {
              window.location.hash = '#/';
            } else {
              window.location.hash = `#/${category}/${section}${page ? `/${page}` : ''}`;
            }
          }}
        />
        <TableOfContents content={content()} />
      </div>
    </div>
  );
};

export default App;

