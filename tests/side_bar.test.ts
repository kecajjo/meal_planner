import { test, expect } from '@playwright/test';

const basePath = '/';

test.describe('Sidebar on desktop', () => {
  test.use({ viewport: { width: 1280, height: 720 } });

  test('shows sidebar and switches views', async ({ page }) => {
    await page.goto(basePath);

    const sidebar = page.locator('.action-bar');
    await expect(sidebar).toBeVisible();

    await page.getByRole('button', { name: 'Swap Foods' }).click();
    await expect(page.getByText('Swap Food View')).toBeVisible();

    await page.getByRole('button', { name: 'Meal Plan' }).click();
    await expect(page.getByText('Meal Plan View')).toBeVisible();
  });

  test('sidebar is always open on desktop', async ({ page }) => {
    await page.goto(basePath);

    const sidebar = page.locator('.action-bar');
    await expect(sidebar).toBeVisible();

    const handle = page.locator('.sidebar-handle');
    await expect(handle).not.toBeVisible();
  });
});

test.describe('Sidebar on mobile', () => {
  test.use({ viewport: { width: 390, height: 844 } }); // iPhone 12-ish size

  test('can open and close via handle', async ({ page }) => {
    await page.goto(basePath);

    const sidebar = page.locator('.action-bar');
    const handle = page.locator('.sidebar-handle');

    await expect(handle).toBeVisible();
    await expect(sidebar).not.toHaveClass(/action-bar--open/);

    // Tap/click the handle to open
    await handle.click({ position: { x: 5, y: 30 } });
    await expect(sidebar).toHaveClass(/action-bar--open/);

    await page.getByRole('button', { name: 'Swap Foods' }).click();
    await expect(page.getByText('Swap Food View')).toBeVisible();

    // Clicking handle again should not close if design is toggle-on-open only
    await handle.click({ position: { x: 5, y: 30 } });
    await expect(sidebar).toHaveClass(/action-bar--open/);

    await page.getByRole('button', { name: 'Close navigation' }).click();
    await expect(sidebar).not.toHaveClass(/action-bar--open/);
  });

  test('can open and close via swipe', async ({ page }) => {
    await page.goto(basePath);

    const sidebar = page.locator('.action-bar');
    const handle = page.locator('.sidebar-handle');

    await expect(handle).toBeVisible();
    await expect(sidebar).not.toHaveClass(/action-bar--open/);

    // Swipe right to open (simulate drag from left edge)
    await handle.hover({ position: { x: 6, y: 30 } });
    await page.mouse.down();
    await page.mouse.move(180, 100, { steps: 12 });
    await page.mouse.up();
    await expect(sidebar).toHaveClass(/action-bar--open/);

    // Swipe left to close (simulate drag on sidebar)
    const sidebarBox = await sidebar.boundingBox();
    if (sidebarBox) {
      const { x, y, width, height } = sidebarBox;
      // click anywhere to unselect all the text it celected in this test
      await page.mouse.click(x + width + 10, y + height / 2);
      await sidebar.hover({ position: { x: width - 10, y: height / 2 } });
      await page.mouse.down();
      // Fix: move relative to sidebar's x, not absolute 0
      await page.mouse.move(x - 80, y + height / 2, { steps: 12 });
      await page.mouse.up();
    }
    await expect(sidebar).not.toHaveClass(/action-bar--open/);
  });

  test('sidebar remains closed after failed open gesture', async ({ page }) => {
    await page.goto(basePath);

    const sidebar = page.locator('.action-bar');
    const handle = page.locator('.sidebar-handle');

    await expect(handle).toBeVisible();
    await expect(sidebar).not.toHaveClass(/action-bar--open/);

    // Short swipe (should not open)
    await handle.hover({ position: { x: 6, y: 30 } });
    await page.mouse.down();
    await page.mouse.move(20, 30, { steps: 2 });
    await page.waitForTimeout(1100);
    await page.mouse.up();
    await expect(sidebar).not.toHaveClass(/action-bar--open/);
  });

  test('sidebar closes when clicking backdrop', async ({ page }) => {
    await page.goto(basePath);

    const sidebar = page.locator('.action-bar');
    const handle = page.locator('.sidebar-handle');

    // Open sidebar
    await handle.click({ position: { x: 5, y: 30 } });
    await expect(sidebar).toHaveClass(/action-bar--open/);

    // Click backdrop (assuming .sidebar-backdrop exists)
    const backdrop = page.locator('.sidebar-backdrop');
    if (await backdrop.isVisible()) {
      await backdrop.click({ position: { x: 10, y: 10 } });
      await expect(sidebar).not.toHaveClass(/action-bar--open/);
    }
  });

  test('sidebar does not open when swiping in the middle of the screen', async ({ page }) => {
    await page.goto(basePath);

    const sidebar = page.locator('.action-bar');

    // Simulate swipe in the middle of the screen
    await page.mouse.move(200, 400);
    await page.mouse.down();
    await page.mouse.move(350, 400, { steps: 10 });
    await page.mouse.up();

    await expect(sidebar).not.toHaveClass(/action-bar--open/);
  });
});
