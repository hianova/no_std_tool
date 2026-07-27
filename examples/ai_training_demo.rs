use no_std_tool::linalg::Matrix;
use no_std_tool::linalg::calculus::{gradient_descent_step, mse_loss};
use no_std_tool::random::Xoshiro256StarStar;

fn main() {
    println!("==================================================");
    println!("    🧠 Zero-Allocation AI Training Engine 🧠    ");
    println!("==================================================");
    println!();

    // ----------------------------------------------------
    // Demo 1: Linear Algebra (Matrix Multiplication)
    // ----------------------------------------------------
    println!("[1] Linear Algebra (Matrix Multiplication & Transpose)");
    
    // A: 2x3 Matrix
    let a = Matrix::<2, 3>::new([
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0]
    ]);
    // B: 3x2 Matrix
    let b = Matrix::<3, 2>::new([
        [7.0, 8.0],
        [9.0, 1.0],
        [2.0, 3.0]
    ]);
    
    // Dot Product: A * B
    let c = a * b;
    println!("Matrix A (2x3):\n{}", a);
    println!("Matrix B (3x2):\n{}", b);
    println!("A * B = C (2x2):\n{}", c);
    
    // ----------------------------------------------------
    // Demo 2: Calculus (Gradient Descent)
    // ----------------------------------------------------
    println!("\n[2] Calculus: Training a Neuron via Gradient Descent");
    
    // Target Function: y = 2.5 * x1 - 1.5 * x2
    // We want the AI to "learn" the weights [2.5, -1.5]
    
    // Inputs: 4 samples, 2 features
    let x_train = Matrix::<4, 2>::new([
        [1.0, 2.0],
        [2.0, 1.0],
        [-1.0, 3.0],
        [4.0, -2.0]
    ]);
    
    // True Outputs (calculated from target function)
    let y_true = Matrix::<4, 1>::new([
        [-0.5],  // 2.5(1) - 1.5(2)
        [3.5],   // 2.5(2) - 1.5(1)
        [-7.0],  // 2.5(-1) - 1.5(3)
        [13.0]   // 2.5(4) - 1.5(-2)
    ]);
    
    // Initialize Weights randomly using no_std_tool
    let mut rng = Xoshiro256StarStar::from_entropy();
    // Random weights between -1.0 and 1.0
    let w1 = (rng.next_u64() % 1000) as f32 / 500.0 - 1.0;
    let w2 = (rng.next_u64() % 1000) as f32 / 500.0 - 1.0;
    let mut weights = Matrix::<2, 1>::new([[w1], [w2]]);
    
    println!("Target Weights: \n[ 2.5000, -1.5000]");
    println!("Initial Random Weights: \n{}", weights.transpose());
    
    let learning_rate = 0.05;
    let epochs = 100;
    
    println!("Starting Training ({} epochs, LR={})...\n", epochs, learning_rate);
    
    for epoch in 0..=epochs {
        // Forward Pass: y_pred = X * W
        let y_pred = x_train * weights;
        
        // Calculate Loss
        let loss = mse_loss(&y_pred, &y_true);
        
        if epoch % 20 == 0 {
            println!("  Epoch {:3} | Loss: {:>8.4} | Weights: [{:>7.4}, {:>7.4}]", 
                epoch, loss, weights.data[0][0], weights.data[1][0]);
        }
        
        // Calculus: Backpropagation & Weight Update
        gradient_descent_step(&x_train, &mut weights, &y_pred, &y_true, learning_rate);
    }
    
    println!("\n✅ Training Complete!");
    println!("Final Learned Weights:\n{}", weights);
    println!("==================================================");
}
