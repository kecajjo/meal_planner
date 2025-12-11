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
    await handle.click({ position: { x: 10, y: 30 } });
    await expect(sidebar).toHaveClass(/action-bar--open/);

    await page.getByRole('button', { name: 'Swap Foods' }).click();
    await expect(page.getByText('Swap Food View')).toBeVisible();

    await page.getByRole('button', { name: 'Close navigation' }).click();
    await expect(sidebar).not.toHaveClass(/action-bar--open/);
  });
});
