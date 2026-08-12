const { chromium } = require('@playwright/test');
const { readFileSync, writeFileSync, mkdirSync } = require('fs');
const { join } = require('path');

const TEST_CASES_FILE = join(__dirname, 'ui-test-cases.json');
const SCREENSHOT_DIR = join(__dirname, 'screenshots');

// テストケースの読み込み
const testCases = JSON.parse(readFileSync(TEST_CASES_FILE, 'utf-8'));

// スクリーンショットディレクトリの作成
try {
  mkdirSync(SCREENSHOT_DIR, { recursive: true });
} catch (e) {
  // Already exists
}

// アサーション実行
async function executeAssertion(page, assertion) {
  const element = await page.locator(assertion.selector).first();
  
  switch (assertion.type) {
    case 'element':
      if (assertion.exists !== undefined) {
        const count = await element.count();
        if (assertion.exists && count === 0) {
          throw new Error(`Expected element to exist but found none: ${assertion.selector}`);
        }
        if (!assertion.exists && count > 0) {
          throw new Error(`Expected element to not exist but found: ${assertion.selector}`);
        }
      }
      if (assertion.text) {
        const actualText = await element.textContent();
        if (!actualText.includes(assertion.text)) {
          throw new Error(`Expected text "${assertion.text}" but got "${actualText}": ${assertion.selector}`);
        }
      }
      break;
      
    case 'button':
      const button = await page.locator('button').first();
      if (assertion.exists !== undefined) {
        const count = await button.count();
        if (assertion.exists && count === 0) {
          throw new Error(`Expected button to exist but found none`);
        }
        if (!assertion.exists && count > 0) {
          throw new Error(`Expected button to not exist but found`);
        }
      }
      if (assertion.text) {
        const buttonText = await button.textContent();
        if (!buttonText.includes(assertion.text)) {
          throw new Error(`Expected button text "${assertion.text}" but got "${buttonText}"`);
        }
      }
      break;
      
    case 'text':
      const textElement = await page.locator(assertion.selector).first();
      const textContent = await textElement.textContent();
      if (assertion.contains && !textContent.includes(assertion.contains)) {
        throw new Error(`Expected text to contain "${assertion.contains}" but got "${textContent}": ${assertion.selector}`);
      }
      break;
      
    case 'status-pill':
      const statusPill = await page.locator('[data-status]').first();
      if (assertion.exists !== undefined) {
        const count = await statusPill.count();
        if (assertion.exists && count === 0) {
          throw new Error(`Expected status pill to exist but found none`);
        }
        if (!assertion.exists && count > 0) {
          throw new Error(`Expected status pill to not exist but found`);
        }
      }
      break;
  }
}

// ナビゲーション実行
async function executeNavigation(page, navigation) {
  switch (navigation.action) {
    case 'click':
      await page.locator(navigation.selector).first().click();
      break;
    case 'navigate':
      await page.goto(navigation.url);
      break;
    case 'fill':
      await page.locator(navigation.selector).first().fill(navigation.value);
      break;
  }
}

// スクリーンショット撮影
async function takeScreenshot(page, testScreen, isFailure = false) {
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const screenshotName = `${testScreen}-${isFailure ? 'error' : 'success'}-${timestamp}.png`;
  const screenshotPath = join(SCREENSHOT_DIR, screenshotName);
  
  await page.screenshot({ 
    path: screenshotPath,
    fullPage: false 
  });
  
  console.log(`Screenshot saved: ${screenshotPath}`);
  return screenshotPath;
}

// テスト実行
async function runTest(screenName, screenConfig) {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  try {
    // URLにナビゲート
    await page.goto(screenConfig.url);
    
    // ナビゲーションアクションを実行
    for (const nav of screenConfig.navigations || []) {
      await executeNavigation(page, nav);
      await page.waitForTimeout(500); // ナビゲーション後の待機
    }
    
    // アサーションを実行
    for (const assertion of screenConfig.assertions) {
      await executeAssertion(page, assertion);
    }
    
    // 成功時のスクリーンショット撮影
    if (screenConfig.screenshot?.onSuccess) {
      await takeScreenshot(page, screenName, false);
    }
    
    console.log(`✓ Test passed: ${screenConfig.title}`);
    return true;
    
  } catch (error) {
    // 失敗時のスクリーンショット撮影
    if (screenConfig.screenshot?.onFailure) {
      await takeScreenshot(page, screenName, true);
    }
    
    console.error(`✗ Test failed: ${screenConfig.title}`);
    console.error(`  Error: ${error.message}`);
    throw error;
    
  } finally {
    await browser.close();
  }
}

// メイン処理
async function main() {
  const args = process.argv.slice(2);
  const testScreen = args[0]; // テストケース名（空の場合は全て）
  
  console.log('Starting UI tests...\n');
  
  let passed = 0;
  let failed = 0;
  const failedScreens = [];
  
  if (testScreen) {
    // 指定されたテストケースのみ実行
    if (testCases.screens[testScreen]) {
      try {
        await runTest(testScreen, testCases.screens[testScreen]);
        passed++;
      } catch (error) {
        failed++;
        failedScreens.push(testScreen);
      }
    } else {
      console.error(`Test case "${testScreen}" not found`);
      process.exit(1);
    }
  } else {
    // 全てのテストケースを実行
    for (const [screenName, screenConfig] of Object.entries(testCases.screens)) {
      try {
        await runTest(screenName, screenConfig);
        passed++;
      } catch (error) {
        failed++;
        failedScreens.push(screenName);
      }
    }
  }
  
  // 結果を出力
  console.log('\n========== Test Results ==========');
  console.log(`Total: ${passed + failed}`);
  console.log(`Passed: ${passed}`);
  console.log(`Failed: ${failed}`);
  
  if (failedScreens.length > 0) {
    console.log('\nFailed screens:');
    failedScreens.forEach(screen => console.log(`  - ${screen}`));
  }
  
  // 失敗した場合はエラー終了
  if (failed > 0) {
    process.exit(1);
  }
}

main().catch(error => {
  console.error('Unexpected error:', error);
  process.exit(1);
});